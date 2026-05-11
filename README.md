# YT-HOME

[![Release](https://img.shields.io/github/v/release/YTjungle666/YT-HOME?display_name=tag)](https://github.com/YTjungle666/YT-HOME/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/YTjungle666/YT-HOME/ci.yml?branch=main&label=ci)](https://github.com/YTjungle666/YT-HOME/actions/workflows/ci.yml)
[![Docker](https://img.shields.io/github/actions/workflow/status/YTjungle666/YT-HOME/docker.yml?branch=main&label=docker)](https://github.com/YTjungle666/YT-HOME/actions/workflows/docker.yml)
[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)

`YT-HOME` 是一个面向家庭网络回家场景的 `sing-box` 控制面板。
它把入站、客户端、二维码、订阅、TLS / Reality、运行状态和访问边界统一到一个中文面板里，适合部署在家庭服务器、`PVE`、`NAS` 和小主机上。

## 它解决什么问题

- 不再手工拼回家节点配置
- 用一个面板管理入站、客户端、订阅和二维码
- 明确区分普通公网访问节点与“代理回家”节点
- 让手机、平板、电脑快速导入并稳定使用

## 你会得到什么

- 简体中文界面
- 适合家庭回家场景的 `sing-box` 控制面
- 可直接导入客户端的订阅、链接与二维码
- 默认收紧的访问边界控制
- Rust 后端带来的更稳资源占用和更清晰的结构

## 技术栈

- 后端：Rust
- 前端：Vue 3 + Vuetify
- 运行核心：`sing-box`

## 默认信息

- 面板地址：`http://<你的地址>/`
- 订阅地址：`http://<你的地址>:2096/sub/`
- 默认账号：`admin`
- 默认密码：`admin`
- 当前版本：`v3.0.1`
- 当前发布平台：`linux/amd64`
- 默认 `sing-box`：`1.13.11`，源码构建并启用 `with_v2ray_api`

## 部署方式 1：一键安装

适合已经有 Linux 主机，希望几分钟内装好就开始用。

直接安装最新版：

```bash
bash <(curl -Ls https://raw.githubusercontent.com/YTjungle666/YT-HOME/main/install.sh)
```

安装指定版本：

```bash
bash <(curl -Ls https://raw.githubusercontent.com/YTjungle666/YT-HOME/main/install.sh) v3.0.1
```

说明：

- 支持 `systemd` 和 `OpenRC`
- Alpine 请先保证系统里有 `bash`
- 安装完成后直接访问面板地址即可

## 部署方式 2：Docker / GHCR 镜像

适合已经用 Docker 管理服务的环境。

镜像地址：

```text
ghcr.io/ytjungle666/yt-home
```

直接运行：

```bash
docker run -d \
  --name yt-home \
  --restart unless-stopped \
  -p 80:80 \
  -p 2096:2096 \
  -v $(pwd)/db:/app/db \
  ghcr.io/ytjungle666/yt-home:latest
```

使用 Compose：

```bash
mkdir -p yt-home && cd yt-home
curl -LO https://raw.githubusercontent.com/YTjungle666/YT-HOME/main/docker-compose.yml
docker compose up -d
```

### 容器 SSH 访问

Docker / CT 镜像内置 OpenSSH，但默认不启动。只有设置 `YTHOME_ENABLE_SSH=1` 时才会在容器内启动 `sshd`。

- `YTHOME_SSH_PUBLIC_KEY`：写入一个或多个公钥，多个公钥可用换行分隔
- `YTHOME_SSH_AUTHORIZED_KEYS`：指向容器内已有的 authorized_keys 文件
- `YTHOME_SSH_PASSWORD_LOGIN=1`：允许密码登录；默认关闭密码登录，只允许密钥登录

示例：

```bash
docker run -d \
  --name yt-home \
  --restart unless-stopped \
  -p 80:80 \
  -p 2096:2096 \
  -p 2222:22 \
  -e YTHOME_ENABLE_SSH=1 \
  -e YTHOME_SSH_PUBLIC_KEY="$(cat ~/.ssh/id_ed25519.pub)" \
  -v $(pwd)/db:/app/db \
  ghcr.io/ytjungle666/yt-home:latest
```

如果在嵌套 Docker / 受限 PVE 环境里连接后立刻断开，说明容器安全策略拦截了 OpenSSH 的会话重执行；请改用 PVE CT 运行，或给 Docker 容器增加允许 OpenSSH 的 seccomp/privileged 配置。

## 部署方式 3：PVE CT 模板

适合已经习惯在 `PVE LXC/CT` 里直接跑服务的环境。

Release 页面会直接提供可创建 CT 的 rootfs 包：

```text
yt-home-ct-amd64-rootfs.tar.gz
```

创建示例：

```bash
pct create 210 local:vztmpl/yt-home-ct-amd64-rootfs.tar.gz \
  --hostname yt-home \
  --cores 2 \
  --memory 1024 \
  --rootfs local-lvm:8 \
  --net0 name=eth0,bridge=vmbr0,ip=dhcp
```

启动：

```bash
pct start 210
```

这个 rootfs 已经内置 CT 启动入口，创建后可直接启动，不依赖额外容器运行时。

## 修改管理员账号密码

默认账号和密码都是 `admin`。首次部署后建议立即修改。

面板中的“账号安全”页面可以直接修改当前管理员用户名和密码，也可以退出登录。修改后当前会话会失效，需要重新登录。

### 通过管理脚本修改

普通安装会把管理脚本安装到 `/usr/bin/YT-HOME`：

```bash
sudo YT-HOME
```

进入菜单后选择 `6` 设置管理员账号密码，选择 `7` 查看当前管理员信息。

也可以直接执行：

```bash
sudo /usr/local/YT-HOME/YTHOME admin -username '新账号' -password '新密码'
sudo /usr/local/YT-HOME/YTHOME admin -show
```

### 普通二进制安装修改

如果是手动解压 Release 包，进入实际安装目录后执行内置二进制：

```bash
cd /usr/local/YT-HOME
sudo ./YTHOME admin -username '新账号' -password '新密码'
sudo ./YTHOME admin -show
```

如果你自定义了数据库目录，需要带上同一个环境变量：

```bash
sudo YTHOME_DB_FOLDER=/你的数据库目录 /usr/local/YT-HOME/YTHOME admin -username '新账号' -password '新密码'
```

### Docker 部署修改

Docker 镜像里的二进制路径是 `/app/YTHOME`：

```bash
docker exec -it yt-home /app/YTHOME admin -username '新账号' -password '新密码'
docker exec -it yt-home /app/YTHOME admin -show
```

如果 Compose 服务名被用来执行，也可以：

```bash
docker compose exec YT-HOME /app/YTHOME admin -username '新账号' -password '新密码'
docker compose exec YT-HOME /app/YTHOME admin -show
```

如果容器自定义了数据库目录，同样需要传入 `YTHOME_DB_FOLDER`。

## 发布产物

- Linux 安装包：`YT-HOME-linux-amd64.tar.gz`
- PVE CT rootfs：`yt-home-ct-amd64-rootfs.tar.gz`
- Docker 镜像：`ghcr.io/ytjungle666/yt-home`
- Release 页面：<https://github.com/YTjungle666/YT-HOME/releases>

## sing-box 与流量统计

默认发布产物会从上游 `SagerNet/sing-box` 源码标签 `v1.13.11` 构建运行核心，使用上游默认 release build tags、额外启用 `with_v2ray_api`，并在 Linux 上默认采用 purego/CGO-disabled 构建以避免拉取大型 naive/cronet CGO 工具链。这个能力用于读取 `sing-box` V2Ray StatsService，提供客户端、入站和出站的上传/下载统计。

- 构建时可用 `SING_BOX_VERSION` 覆盖源码标签，但运行统计仍要求构建结果的 `sing-box version` 输出包含 `with_v2ray_api`
- `YTHOME_V2RAY_API_LISTEN` 默认是本机地址 `127.0.0.1:21085`
- 设置 `YTHOME_V2RAY_API_LISTEN=off`、`0`、`false` 或 `disabled` 会关闭 StatsService 注入
- 官方 `sing-box` release 二进制通常没有 `with_v2ray_api`；如果用 `YTHOME_SING_BOX_BIN` 指向自定义二进制，请先执行 `sing-box version | grep -F with_v2ray_api`
- 在线用户状态基于最近流量增量推断，不等同于当前 TCP 连接列表
- 流量配额按客户端上传加下载总量计算，也就是 `client.up + client.down`；入站、出站和用户统计是同一份流量的不同视图，不会相互累加用于配额

## 使用建议

- 面板保留主页、用户、入站、TLS、订阅、设置和账号安全等家庭回家常用功能
- 出站、节点、服务、基础配置、路由和 DNS 等历史高级配置页面已从前端隐藏，对应 sing-box 运行配置由后端写入安全默认值
- 手机和下游客户端订阅、链接导入、二维码流程仍然保留
- 面板部署在内网服务器，公网只开放必要端口
- 只有确实需要回家访问的入站才开启“代理回家”
- Reality 节点建议使用你自己可控的域名
- 首次登录后按自己的环境修改面板端口、订阅地址和域名

## 许可证

`YT-HOME` 是基于家庭回家访问场景重新整理的 Rust / Vue 项目，项目本体按 `GPL-3.0-only` 发布。仓库根目录保留未经修改的 GPLv3 正文，方便 GitHub 和下游用户识别授权。

前端与运维脚本中继承自 GPL 项目历史的部分继续按 GPL 授权，YT-HOME 的新增与整理内容也随整个项目按 `GPL-3.0-only` 分发；第三方依赖、字体、`sing-box` 以及 `vendor/`、`patches/` 目录中的组件继续遵循各自许可证。详细说明见 `NOTICE`。
