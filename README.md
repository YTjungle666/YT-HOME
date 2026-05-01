# YT-HOME

![Release](https://img.shields.io/github/v/release/YTjungle666/YT-HOME?display_name=tag)
![CI](https://img.shields.io/github/actions/workflow/status/YTjungle666/YT-HOME/ci.yml?branch=main&label=ci)
![Docker](https://img.shields.io/github/actions/workflow/status/YTjungle666/YT-HOME/docker.yml?branch=main&label=docker)
![License](https://img.shields.io/github/license/YTjungle666/YT-HOME)

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
- 当前版本：`v2.0.8`
- 当前发布平台：`linux/amd64`

## 部署方式 1：一键安装

适合已经有 Linux 主机，希望几分钟内装好就开始用。

直接安装最新版：

```bash
bash <(curl -Ls https://raw.githubusercontent.com/YTjungle666/YT-HOME/main/install.sh)
```

安装指定版本：

```bash
bash <(curl -Ls https://raw.githubusercontent.com/YTjungle666/YT-HOME/main/install.sh) v2.0.8
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

## 使用建议

- 面板部署在内网服务器，公网只开放必要端口
- 只有确实需要回家访问的入站才开启“代理回家”
- Reality 节点建议使用你自己可控的域名
- 首次登录后按自己的环境修改面板端口、订阅地址和域名

## 许可证

`YT-HOME` 是基于家庭回家访问场景重新整理的 Rust / Vue 项目，项目本体按 `GPL-3.0-only` 发布。第三方依赖、`sing-box` 以及 vendor/patch 目录中的组件继续遵循各自许可证。
