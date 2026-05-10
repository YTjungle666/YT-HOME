use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    sync::Arc,
    time::Duration,
};

use domain_core::CoreService;
use if_addrs::{IfAddr, get_if_addrs};
use infra_db::Db;
use serde_json::{Map, Value, json};
use shared::{AppError, AppResult, settings::APP_VERSION};
use sqlx::Row;
use sysinfo::{Disks, Networks, System, get_current_pid};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::debug;

const ONLINE_WINDOW_SECS: i64 = 180;
const DEFAULT_STATS_HOURS: i64 = 24;
const MAX_STATS_HOURS: i64 = 2_160;
const MAX_STATS_ROWS: i64 = 20_000;
const MAX_TRAFFIC_AGE_DAYS: i64 = 3_650;
const V2RAY_STATS_TIMEOUT: Duration = Duration::from_secs(3);
const V2RAY_STATS_PATH: &str = "/v2ray.core.app.stats.command.StatsService/QueryStats";

#[derive(Debug)]
struct RuntimeStats {
    system: System,
    disks: Disks,
    networks: Networks,
}

impl RuntimeStats {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }
}

#[derive(Debug, Default)]
struct TrafficCounters {
    values: BTreeMap<String, u64>,
}

#[derive(Clone)]
pub struct StatsService {
    pool: Db,
    runtime: Arc<RwLock<RuntimeStats>>,
    runtime_counters: Arc<RwLock<TrafficCounters>>,
    app_counters: Arc<RwLock<TrafficCounters>>,
}

impl StatsService {
    pub fn new(pool: Db) -> Self {
        Self {
            pool,
            runtime: Arc::new(RwLock::new(RuntimeStats::new())),
            runtime_counters: Arc::new(RwLock::new(TrafficCounters::default())),
            app_counters: Arc::new(RwLock::new(TrafficCounters::default())),
        }
    }

    pub async fn get_onlines(&self) -> AppResult<Value> {
        let since = OffsetDateTime::now_utc().unix_timestamp().saturating_sub(ONLINE_WINDOW_SECS);
        let rows = sqlx::query(
            r#"
            SELECT resource, tag, SUM(traffic) AS traffic
            FROM stats
            WHERE date_time >= ? AND traffic > 0
            GROUP BY resource, tag
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        let rows = rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("resource"),
                    row.get::<String, _>("tag"),
                    row.get::<i64, _>("traffic"),
                )
            })
            .collect::<Vec<_>>();
        Ok(online_snapshot_from_rows(rows))
    }

    pub async fn get_stats(
        &self,
        resource: Option<&str>,
        tag: Option<&str>,
        hours: i64,
    ) -> AppResult<Vec<Value>> {
        let hours = clamp_stats_hours(hours);
        let since =
            OffsetDateTime::now_utc().unix_timestamp().saturating_sub(hours.saturating_mul(3_600));
        let mut sql = String::from(
            "SELECT id, date_time, resource, tag, direction, traffic FROM stats WHERE date_time >= ?",
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(resource) = resource.filter(|value| !value.is_empty()) {
            sql.push_str(" AND resource = ?");
            binds.push(resource.to_string());
        }
        if let Some(tag) = tag.filter(|value| !value.is_empty()) {
            sql.push_str(" AND tag = ?");
            binds.push(tag.to_string());
        }
        sql.push_str(" ORDER BY date_time DESC, id DESC LIMIT ?");

        let mut query = sqlx::query(&sql).bind(since);
        for bind in binds {
            query = query.bind(bind);
        }
        let rows = query.bind(MAX_STATS_ROWS).fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                json!({
                    "id": row.get::<i64, _>("id"),
                    "dateTime": row.get::<i64, _>("date_time"),
                    "resource": row.get::<String, _>("resource"),
                    "tag": row.get::<String, _>("tag"),
                    "direction": row.get::<bool, _>("direction"),
                    "traffic": row.get::<i64, _>("traffic"),
                })
            })
            .collect())
    }

    pub async fn collect_runtime_traffic(
        &self,
        core: &CoreService,
        traffic_age_days: i64,
    ) -> AppResult<()> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        if traffic_age_days <= 0 {
            self.reset_traffic_collection().await?;
            return Ok(());
        }

        self.prune_old_stats(now, traffic_age_days).await?;

        let Some(config) = core.current_config().await else {
            self.collect_client_counter_deltas(now).await?;
            return Ok(());
        };
        let Some(api) = runtime_stats_api_from_config(&config) else {
            self.collect_client_counter_deltas(now).await?;
            return Ok(());
        };

        match query_v2ray_stats(&api.listen).await {
            Ok(counters) => self.store_runtime_counter_deltas(now, counters).await,
            Err(error) => {
                debug!("runtime stats API collection skipped: {}", error.message());
                Ok(())
            }
        }
    }

    async fn reset_traffic_collection(&self) -> AppResult<()> {
        sqlx::query("DELETE FROM stats").execute(&self.pool).await?;
        self.runtime_counters.write().await.values.clear();
        self.app_counters.write().await.values.clear();
        Ok(())
    }

    async fn prune_old_stats(&self, now: i64, traffic_age_days: i64) -> AppResult<()> {
        let retention_days = traffic_age_days.clamp(1, MAX_TRAFFIC_AGE_DAYS);
        let since = now.saturating_sub(retention_days.saturating_mul(86_400));
        sqlx::query("DELETE FROM stats WHERE date_time < ?")
            .bind(since)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn store_runtime_counter_deltas(
        &self,
        now: i64,
        counters: BTreeMap<String, u64>,
    ) -> AppResult<()> {
        let deltas = {
            let mut state = self.runtime_counters.write().await;
            counter_deltas(&mut state.values, &counters)
        };
        self.insert_traffic_deltas(now, deltas, true).await
    }

    async fn collect_client_counter_deltas(&self, now: i64) -> AppResult<()> {
        let rows = sqlx::query(
            r#"
            SELECT name, up, down, total_up, total_down
            FROM clients
            WHERE enable = 1
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let mut counters = BTreeMap::new();
        for row in rows {
            let name = row.get::<String, _>("name");
            if name.trim().is_empty() {
                continue;
            }
            let up = non_negative_i64_to_u64(row.get::<i64, _>("up"))
                .saturating_add(non_negative_i64_to_u64(row.get::<i64, _>("total_up")));
            let down = non_negative_i64_to_u64(row.get::<i64, _>("down"))
                .saturating_add(non_negative_i64_to_u64(row.get::<i64, _>("total_down")));
            counters.insert(format!("user>>>{name}>>>traffic>>>uplink"), up);
            counters.insert(format!("user>>>{name}>>>traffic>>>downlink"), down);
        }

        let deltas = {
            let mut state = self.app_counters.write().await;
            counter_deltas(&mut state.values, &counters)
        };
        self.insert_traffic_deltas(now, deltas, false).await
    }

    async fn insert_traffic_deltas(
        &self,
        now: i64,
        deltas: Vec<(String, u64)>,
        update_clients: bool,
    ) -> AppResult<()> {
        let mut rows = Vec::new();
        for (name, delta) in deltas {
            if delta == 0 {
                continue;
            }
            let Some(sample) = parse_v2ray_stat_name(&name, delta) else {
                continue;
            };
            rows.push(sample);
        }
        if rows.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for row in rows {
            sqlx::query(
                "INSERT INTO stats (date_time, resource, tag, direction, traffic) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(now)
            .bind(&row.resource)
            .bind(&row.tag)
            .bind(row.direction)
            .bind(row.traffic)
            .execute(&mut *tx)
            .await?;

            if update_clients && row.resource == "user" {
                let column = if row.direction { "up" } else { "down" };
                let sql = format!("UPDATE clients SET {column} = {column} + ? WHERE name = ?");
                sqlx::query(&sql).bind(row.traffic).bind(&row.tag).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_status(
        &self,
        request: &str,
        db_info: BTreeMap<String, i64>,
        core: &CoreService,
    ) -> Value {
        let mut runtime = self.runtime.write().await;
        runtime.system.refresh_cpu_usage();
        runtime.system.refresh_memory();
        runtime.disks.refresh(false);
        runtime.networks.refresh(false);

        let cpu = runtime.system.global_cpu_usage() as f64;
        let mem = json!({
            "current": runtime.system.used_memory(),
            "total": runtime.system.total_memory(),
        });
        let swap = json!({
            "current": runtime.system.used_swap(),
            "total": runtime.system.total_swap(),
        });
        let disk = disk_snapshot(&runtime.disks);
        let disk_io = disk_io_snapshot();
        let net = network_snapshot(&runtime.networks);
        let sys = system_snapshot(&runtime.system);

        let mut result = Map::new();
        for item in request.split(',').filter(|item| !item.is_empty()) {
            match item {
                "cpu" => {
                    result.insert("cpu".to_string(), json!(cpu));
                }
                "mem" => {
                    result.insert("mem".to_string(), mem.clone());
                }
                "dsk" => {
                    result.insert("dsk".to_string(), disk.clone());
                }
                "dio" => {
                    result.insert("dio".to_string(), disk_io.clone());
                }
                "swp" => {
                    result.insert("swp".to_string(), swap.clone());
                }
                "net" => {
                    result.insert("net".to_string(), net.clone());
                }
                "sys" => {
                    result.insert("sys".to_string(), sys.clone());
                }
                "sbd" => {
                    result.insert("sbd".to_string(), core.status().await);
                }
                "db" => {
                    result.insert("db".to_string(), json!(db_info));
                }
                _ => {}
            }
        }
        Value::Object(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeStatsApi {
    listen: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrafficSample {
    resource: String,
    tag: String,
    direction: bool,
    traffic: i64,
}

fn clamp_stats_hours(hours: i64) -> i64 {
    if hours <= 0 { DEFAULT_STATS_HOURS } else { hours.min(MAX_STATS_HOURS) }
}

fn runtime_stats_api_from_config(config: &str) -> Option<RuntimeStatsApi> {
    let root: Value = serde_json::from_str(config).ok()?;
    let api = root.get("experimental")?.get("v2ray_api")?;
    let listen = api.get("listen")?.as_str()?.trim();
    if listen.is_empty() {
        return None;
    }
    let stats = api.get("stats")?;
    if stats.get("enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    Some(RuntimeStatsApi { listen: listen.to_string() })
}

async fn query_v2ray_stats(listen: &str) -> AppResult<BTreeMap<String, u64>> {
    let url = v2ray_stats_url(listen)?;
    let body = encode_grpc_message(&encode_query_stats_request("", false));
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(V2RAY_STATS_TIMEOUT)
        .build()
        .map_err(|error| AppError::Unsupported(error.to_string()))?;
    let response = client
        .post(url)
        .header("content-type", "application/grpc")
        .header("te", "trailers")
        .body(body)
        .send()
        .await
        .map_err(|error| AppError::Unsupported(error.to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Unsupported(format!(
            "stats API returned HTTP {}",
            response.status()
        )));
    }
    let body = response.bytes().await.map_err(|error| AppError::Unsupported(error.to_string()))?;
    let messages = decode_grpc_messages(&body)?;
    let mut counters = BTreeMap::new();
    for message in messages {
        counters.extend(decode_query_stats_response(&message)?);
    }
    Ok(counters)
}

fn v2ray_stats_url(listen: &str) -> AppResult<String> {
    let listen = listen.trim();
    if listen.is_empty() {
        return Err(AppError::Validation("stats API listen address is empty".to_string()));
    }
    let base = if listen.starts_with("http://") || listen.starts_with("https://") {
        listen.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", listen.trim_end_matches('/'))
    };
    Ok(format!("{base}{V2RAY_STATS_PATH}"))
}

fn encode_query_stats_request(pattern: &str, reset: bool) -> Vec<u8> {
    let mut output = Vec::new();
    if !pattern.is_empty() {
        output.push(0x0a);
        encode_varint(pattern.len() as u64, &mut output);
        output.extend_from_slice(pattern.as_bytes());
    }
    if reset {
        output.push(0x10);
        output.push(1);
    }
    output
}

fn encode_grpc_message(message: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(message.len() + 5);
    output.push(0);
    output.extend_from_slice(&(message.len() as u32).to_be_bytes());
    output.extend_from_slice(message);
    output
}

fn decode_grpc_messages(mut input: &[u8]) -> AppResult<Vec<Vec<u8>>> {
    let mut messages = Vec::new();
    while !input.is_empty() {
        if input.len() < 5 {
            return Err(AppError::Validation("truncated gRPC stats frame".to_string()));
        }
        let compressed = input[0];
        if compressed != 0 {
            return Err(AppError::Unsupported(
                "compressed gRPC stats frames are not supported".to_string(),
            ));
        }
        let len = u32::from_be_bytes([input[1], input[2], input[3], input[4]]) as usize;
        input = &input[5..];
        if input.len() < len {
            return Err(AppError::Validation("truncated gRPC stats message".to_string()));
        }
        messages.push(input[..len].to_vec());
        input = &input[len..];
    }
    Ok(messages)
}

fn decode_query_stats_response(mut input: &[u8]) -> AppResult<BTreeMap<String, u64>> {
    let mut counters = BTreeMap::new();
    while !input.is_empty() {
        let key = read_varint(&mut input)?;
        let field = key >> 3;
        let wire = key & 0x07;
        if field == 1 && wire == 2 {
            let stat = read_length_delimited(&mut input)?;
            if let Some((name, value)) = decode_stat(stat)? {
                counters.insert(name, value);
            }
        } else {
            skip_wire_value(wire, &mut input)?;
        }
    }
    Ok(counters)
}

fn decode_stat(mut input: &[u8]) -> AppResult<Option<(String, u64)>> {
    let mut name = None;
    let mut value = None;
    while !input.is_empty() {
        let key = read_varint(&mut input)?;
        let field = key >> 3;
        let wire = key & 0x07;
        match (field, wire) {
            (1, 2) => {
                let raw = read_length_delimited(&mut input)?;
                name = Some(
                    std::str::from_utf8(raw)
                        .map_err(|error| AppError::Validation(error.to_string()))?
                        .to_string(),
                );
            }
            (2, 0) => {
                value = Some(read_varint(&mut input)?);
            }
            _ => skip_wire_value(wire, &mut input)?,
        }
    }
    Ok(name.zip(value))
}

fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push((value as u8) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn read_varint(input: &mut &[u8]) -> AppResult<u64> {
    let mut value = 0_u64;
    for shift in (0..64).step_by(7) {
        let Some((&byte, rest)) = input.split_first() else {
            return Err(AppError::Validation("truncated protobuf varint".to_string()));
        };
        *input = rest;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(AppError::Validation("protobuf varint is too long".to_string()))
}

fn read_length_delimited<'a>(input: &mut &'a [u8]) -> AppResult<&'a [u8]> {
    let len = read_varint(input)? as usize;
    if input.len() < len {
        return Err(AppError::Validation("truncated protobuf field".to_string()));
    }
    let (value, rest) = input.split_at(len);
    *input = rest;
    Ok(value)
}

fn skip_wire_value(wire: u64, input: &mut &[u8]) -> AppResult<()> {
    match wire {
        0 => {
            let _ = read_varint(input)?;
        }
        1 => {
            if input.len() < 8 {
                return Err(AppError::Validation("truncated 64-bit protobuf field".to_string()));
            }
            *input = &input[8..];
        }
        2 => {
            let _ = read_length_delimited(input)?;
        }
        5 => {
            if input.len() < 4 {
                return Err(AppError::Validation("truncated 32-bit protobuf field".to_string()));
            }
            *input = &input[4..];
        }
        _ => return Err(AppError::Unsupported(format!("unsupported protobuf wire type {wire}"))),
    }
    Ok(())
}

fn counter_deltas(
    previous: &mut BTreeMap<String, u64>,
    counters: &BTreeMap<String, u64>,
) -> Vec<(String, u64)> {
    let mut deltas = Vec::new();
    for (name, current) in counters {
        if let Some(previous_value) = previous.insert(name.clone(), *current) {
            let delta = if *current >= previous_value {
                current.saturating_sub(previous_value)
            } else {
                *current
            };
            if delta > 0 {
                deltas.push((name.clone(), delta));
            }
        }
    }
    previous.retain(|name, _| counters.contains_key(name));
    deltas
}

fn parse_v2ray_stat_name(name: &str, traffic: u64) -> Option<TrafficSample> {
    let mut parts = name.split(">>>");
    let resource = normalize_stats_resource(parts.next()?)?;
    let tag = parts.next()?.trim();
    let traffic_part = parts.next()?;
    let direction = match parts.next()? {
        "uplink" => true,
        "downlink" => false,
        _ => return None,
    };
    if parts.next().is_some() || tag.is_empty() || traffic_part != "traffic" {
        return None;
    }
    Some(TrafficSample {
        resource: resource.to_string(),
        tag: tag.to_string(),
        direction,
        traffic: traffic.min(i64::MAX as u64) as i64,
    })
}

fn normalize_stats_resource(resource: &str) -> Option<&'static str> {
    match resource {
        "inbound" | "inbounds" => Some("inbound"),
        "outbound" | "outbounds" => Some("outbound"),
        "user" | "users" | "client" | "clients" => Some("user"),
        _ => None,
    }
}

fn non_negative_i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

fn online_snapshot_from_rows(rows: Vec<(String, String, i64)>) -> Value {
    let mut inbound = BTreeSet::new();
    let mut outbound = BTreeSet::new();
    let mut user = BTreeSet::new();

    for (resource, tag, traffic) in rows {
        if traffic <= 0 || tag.trim().is_empty() {
            continue;
        }
        match resource.as_str() {
            "inbound" | "inbounds" => {
                inbound.insert(tag);
            }
            "outbound" | "outbounds" => {
                outbound.insert(tag);
            }
            "user" | "users" | "client" | "clients" => {
                user.insert(tag);
            }
            _ => {}
        }
    }

    json!({
        "inbound": inbound.into_iter().collect::<Vec<_>>(),
        "outbound": outbound.into_iter().collect::<Vec<_>>(),
        "user": user.into_iter().collect::<Vec<_>>(),
    })
}

fn disk_snapshot(disks: &Disks) -> Value {
    let Some(disk) = disks
        .list()
        .iter()
        .find(|disk| disk.mount_point() == std::path::Path::new("/"))
        .or_else(|| disks.list().iter().max_by_key(|disk| disk.total_space()))
    else {
        return json!({ "current": 0_u64, "total": 0_u64 });
    };

    let total = disk.total_space();
    let current = total.saturating_sub(disk.available_space());
    json!({
        "current": current,
        "total": total,
    })
}

fn network_snapshot(networks: &Networks) -> Value {
    let mut sent = 0_u64;
    let mut recv = 0_u64;
    let mut psent = 0_u64;
    let mut precv = 0_u64;

    for network in networks.list().values() {
        sent = sent.saturating_add(network.total_transmitted());
        recv = recv.saturating_add(network.total_received());
        psent = psent.saturating_add(network.total_packets_transmitted());
        precv = precv.saturating_add(network.total_packets_received());
    }

    json!({
        "sent": sent,
        "recv": recv,
        "psent": psent,
        "precv": precv,
    })
}

fn system_snapshot(system: &System) -> Value {
    let (ipv4, ipv6) = interface_addresses();
    let (app_mem, app_threads) = current_process_snapshot();
    let cpu_type = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let boot_time = instance_boot_time(system);

    json!({
        "appMem": app_mem,
        "appThreads": app_threads,
        "cpuType": cpu_type,
        "cpuCount": system.cpus().len(),
        "hostName": System::host_name().unwrap_or_else(|| "localhost".to_string()),
        "appVersion": APP_VERSION,
        "ipv4": ipv4,
        "ipv6": ipv6,
        "bootTime": boot_time,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProcessTreeNode {
    start_time: u64,
    parent: Option<u32>,
}

fn resolve_instance_boot_time<F>(current_pid: u32, mut lookup: F) -> Option<u64>
where
    F: FnMut(u32) -> Option<ProcessTreeNode>,
{
    let mut pid = current_pid;
    let mut visited = HashSet::new();
    let mut boot_time = None;

    while visited.insert(pid) {
        let Some(node) = lookup(pid) else {
            break;
        };
        if node.start_time > 0 {
            boot_time = Some(
                boot_time.map_or(node.start_time, |current: u64| current.min(node.start_time)),
            );
        }

        match node.parent {
            Some(parent) if parent != pid => pid = parent,
            _ => break,
        }
    }

    boot_time
}

fn instance_boot_time(system: &System) -> u64 {
    let Ok(current_pid) = get_current_pid() else {
        return System::boot_time();
    };

    resolve_instance_boot_time(current_pid.as_u32(), |pid| {
        system.process(sysinfo::Pid::from_u32(pid)).map(|process| ProcessTreeNode {
            start_time: process.start_time(),
            parent: process.parent().map(|parent| parent.as_u32()),
        })
    })
    .or_else(|| {
        system
            .process(current_pid)
            .map(|process| process.start_time())
            .filter(|start_time| *start_time > 0)
    })
    .unwrap_or_else(System::boot_time)
}

fn interface_addresses() -> (Vec<String>, Vec<String>) {
    let Ok(addresses) = get_if_addrs() else {
        return (Vec::new(), Vec::new());
    };

    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();
    for interface in addresses {
        if interface.is_loopback() {
            continue;
        }
        match interface.addr {
            IfAddr::V4(addr) => ipv4.push(addr.ip.to_string()),
            IfAddr::V6(addr) => {
                let ip = addr.ip.to_string();
                if !ip.starts_with("fe80:") {
                    ipv6.push(ip);
                }
            }
        }
    }
    ipv4.sort();
    ipv4.dedup();
    ipv6.sort();
    ipv6.dedup();
    (ipv4, ipv6)
}

fn current_process_snapshot() -> (u64, u64) {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return (0, 0);
    };

    let mut app_mem = 0_u64;
    let mut app_threads = 0_u64;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            app_mem = parse_status_kib(value);
        } else if let Some(value) = line.strip_prefix("Threads:") {
            app_threads = value.trim().parse::<u64>().unwrap_or_default();
        }
    }
    (app_mem, app_threads)
}

fn parse_status_kib(value: &str) -> u64 {
    value
        .split_whitespace()
        .next()
        .and_then(|part| part.parse::<u64>().ok())
        .map(|kib| kib.saturating_mul(1024))
        .unwrap_or_default()
}

fn disk_io_snapshot() -> Value {
    let Ok(content) = fs::read_to_string("/proc/diskstats") else {
        return json!({ "read": 0_u64, "write": 0_u64 });
    };

    let mut read = 0_u64;
    let mut write = 0_u64;
    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 14 {
            continue;
        }
        let Some(name) = fields.get(2) else {
            continue;
        };
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("fd")
            || name.starts_with("sr")
        {
            continue;
        }

        let sectors_read =
            fields.get(5).and_then(|value| value.parse::<u64>().ok()).unwrap_or_default();
        let sectors_written =
            fields.get(9).and_then(|value| value.parse::<u64>().ok()).unwrap_or_default();
        read = read.saturating_add(sectors_read.saturating_mul(512));
        write = write.saturating_add(sectors_written.saturating_mul(512));
    }

    json!({
        "read": read,
        "write": write,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::{
        ProcessTreeNode, StatsService, counter_deltas, decode_grpc_messages,
        decode_query_stats_response, encode_grpc_message, encode_varint, online_snapshot_from_rows,
        resolve_instance_boot_time,
    };

    #[test]
    fn resolve_instance_boot_time_uses_oldest_ancestor_start() {
        let nodes = HashMap::from([
            (301_u32, ProcessTreeNode { start_time: 3000, parent: Some(201) }),
            (201_u32, ProcessTreeNode { start_time: 2000, parent: Some(101) }),
            (101_u32, ProcessTreeNode { start_time: 1000, parent: None }),
        ]);

        let boot_time = resolve_instance_boot_time(301, |pid| nodes.get(&pid).copied());

        assert_eq!(boot_time, Some(1000));
    }

    #[test]
    fn resolve_instance_boot_time_keeps_last_known_when_parent_is_missing() {
        let nodes = HashMap::from([
            (301_u32, ProcessTreeNode { start_time: 3000, parent: Some(201) }),
            (201_u32, ProcessTreeNode { start_time: 2000, parent: Some(101) }),
        ]);

        let boot_time = resolve_instance_boot_time(301, |pid| nodes.get(&pid).copied());

        assert_eq!(boot_time, Some(2000));
    }

    #[test]
    fn resolve_instance_boot_time_breaks_parent_cycles() {
        let nodes = HashMap::from([
            (301_u32, ProcessTreeNode { start_time: 3000, parent: Some(201) }),
            (201_u32, ProcessTreeNode { start_time: 2000, parent: Some(301) }),
        ]);

        let boot_time = resolve_instance_boot_time(301, |pid| nodes.get(&pid).copied());

        assert_eq!(boot_time, Some(2000));
    }

    #[test]
    fn online_snapshot_uses_positive_recent_traffic_rows() {
        let snapshot = online_snapshot_from_rows(vec![
            ("user".to_string(), "alice".to_string(), 128),
            ("users".to_string(), "bob".to_string(), 1),
            ("inbound".to_string(), "vless-in".to_string(), 64),
            ("outbound".to_string(), "direct".to_string(), 0),
            ("unknown".to_string(), "ignored".to_string(), 1),
        ]);

        assert_eq!(snapshot["user"], json!(["alice", "bob"]));
        assert_eq!(snapshot["inbound"], json!(["vless-in"]));
        assert_eq!(snapshot["outbound"], json!([]));
    }

    #[test]
    fn counter_deltas_skip_first_sample_and_handle_resets() {
        let mut previous = BTreeMap::new();
        let first = BTreeMap::from([
            ("user>>>alice>>>traffic>>>uplink".to_string(), 100_u64),
            ("user>>>alice>>>traffic>>>downlink".to_string(), 50_u64),
        ]);
        assert!(counter_deltas(&mut previous, &first).is_empty());

        let second = BTreeMap::from([
            ("user>>>alice>>>traffic>>>uplink".to_string(), 175_u64),
            ("user>>>alice>>>traffic>>>downlink".to_string(), 40_u64),
        ]);
        let deltas = counter_deltas(&mut previous, &second);

        assert_eq!(
            deltas,
            vec![
                ("user>>>alice>>>traffic>>>downlink".to_string(), 40),
                ("user>>>alice>>>traffic>>>uplink".to_string(), 75),
            ]
        );
    }

    #[test]
    fn grpc_query_stats_response_decodes_v2ray_counters() {
        let mut stat = Vec::new();
        stat.push(0x0a);
        let name = "user>>>alice>>>traffic>>>uplink";
        encode_varint(name.len() as u64, &mut stat);
        stat.extend_from_slice(name.as_bytes());
        stat.push(0x10);
        encode_varint(321, &mut stat);

        let mut response = Vec::new();
        response.push(0x0a);
        encode_varint(stat.len() as u64, &mut response);
        response.extend_from_slice(&stat);
        let frame = encode_grpc_message(&response);
        let messages = decode_grpc_messages(&frame).expect("decode grpc frame");
        let counters = decode_query_stats_response(&messages[0]).expect("decode query stats");

        assert_eq!(counters.get(name), Some(&321));
    }

    #[tokio::test]
    async fn get_stats_filters_by_hour_window_not_row_count() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect sqlite");
        sqlx::query(
            r#"
            CREATE TABLE stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date_time INTEGER NOT NULL,
                resource TEXT NOT NULL,
                tag TEXT NOT NULL,
                direction INTEGER NOT NULL,
                traffic INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("create stats");
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        for (date_time, tag, traffic) in [
            (now - 7_200, "alice", 100_i64),
            (now - 1_800, "alice", 200_i64),
            (now - 600, "bob", 300_i64),
        ] {
            sqlx::query(
                "INSERT INTO stats (date_time, resource, tag, direction, traffic) VALUES (?, 'user', ?, 1, ?)",
            )
            .bind(date_time)
            .bind(tag)
            .bind(traffic)
            .execute(&pool)
            .await
            .expect("insert stat");
        }
        let service = StatsService::new(pool);

        let rows = service.get_stats(Some("user"), Some("alice"), 1).await.expect("get stats");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["traffic"], 200);
    }
}
