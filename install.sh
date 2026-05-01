#!/bin/bash

red='\033[0;31m'
green='\033[0;32m'
yellow='\033[0;33m'
plain='\033[0m'
repo="YTjungle666/YT-HOME"

cur_dir=$(pwd)
fresh_install=0

# check root
[[ $EUID -ne 0 ]] && echo -e "${red}Fatal error: ${plain} Please run this script with root privilege \n " && exit 1

# Check OS and set release variable
if [[ -f /etc/os-release ]]; then
    source /etc/os-release
    release=$ID
elif [[ -f /usr/lib/os-release ]]; then
    source /usr/lib/os-release
    release=$ID
else
    echo "Failed to check the system OS, please contact the author!" >&2
    exit 1
fi
echo "The OS release is: $release"

arch() {
    case "$(uname -m)" in
    x86_64 | x64 | amd64) echo 'amd64' ;;
    *) echo -e "${green}Unsupported CPU architecture. Released binaries are available for linux/amd64 only.${plain}" && rm -f install.sh && exit 1 ;;
    esac
}

echo "arch: $(arch)"

is_systemd() {
    command -v systemctl >/dev/null 2>&1 && [[ -d /run/systemd/system ]]
}

is_openrc() {
    command -v rc-service >/dev/null 2>&1
}

ensure_openrc_runtime() {
    mkdir -p /run/openrc
    if [[ ! -f /run/openrc/softlevel ]]; then
        : > /run/openrc/softlevel
    fi
}

stop_service_if_exists() {
    if is_systemd; then
        systemctl stop YT-HOME >/dev/null 2>&1 || true
    elif is_openrc; then
        ensure_openrc_runtime
        rc-service YT-HOME stop >/dev/null 2>&1 || true
    fi
}

install_service_files() {
    if is_systemd; then
        cp -f YT-HOME/*.service /etc/systemd/system/
        systemctl daemon-reload
    elif is_openrc; then
        install -Dm755 YT-HOME/packaging/openrc/YTHOME /etc/init.d/YT-HOME
    else
        echo -e "${red}Unsupported init system. Only systemd and OpenRC are supported.${plain}"
        exit 1
    fi
}

enable_and_start_service() {
    if is_systemd; then
        systemctl enable YT-HOME --now
    elif is_openrc; then
        ensure_openrc_runtime
        rc-update add YT-HOME default >/dev/null 2>&1 || true
        rc-service YT-HOME restart >/dev/null 2>&1 || rc-service YT-HOME start
    else
        echo -e "${red}Unsupported init system. Only systemd and OpenRC are supported.${plain}"
        exit 1
    fi
}

install_base() {
    case "${release}" in
    alpine)
        apk add --no-cache bash curl wget tar tzdata ca-certificates openrc
        ;;
    centos | almalinux | rocky | oracle)
        yum -y update && yum install -y -q wget curl tar tzdata
        ;;
    fedora)
        dnf -y update && dnf install -y -q wget curl tar tzdata
        ;;
    arch | manjaro | parch)
        pacman -Syu && pacman -Syu --noconfirm wget curl tar tzdata
        ;;
    opensuse-tumbleweed)
        zypper refresh && zypper -q install -y wget curl tar timezone
        ;;
    *)
        apt-get update && apt-get install -y -q wget curl tar tzdata
        ;;
    esac
}

config_after_install() {
    echo -e "${yellow}Migration... ${plain}"
    /usr/local/YT-HOME/YTHOME migrate
    
    echo -e "${yellow}Install/update finished! For security it's recommended to modify panel settings ${plain}"
    read -p "Do you want to continue with the modification [y/n]? ": config_confirm
    if [[ "${config_confirm}" == "y" || "${config_confirm}" == "Y" ]]; then
        echo -e "Enter the ${yellow}panel port${plain} (leave blank for existing/default value):"
        read config_port
        echo -e "Enter the ${yellow}panel path${plain} (leave blank for existing/default value):"
        read config_path

        # Sub configuration
        echo -e "Enter the ${yellow}subscription port${plain} (leave blank for existing/default value):"
        read config_subPort
        echo -e "Enter the ${yellow}subscription path${plain} (leave blank for existing/default value):" 
        read config_subPath

        # Set configs
        echo -e "${yellow}Initializing, please wait...${plain}"
        params=""
        [ -z "$config_port" ] || params="$params -port $config_port"
        [ -z "$config_path" ] || params="$params -path $config_path"
        [ -z "$config_subPort" ] || params="$params -subPort $config_subPort"
        [ -z "$config_subPath" ] || params="$params -subPath $config_subPath"
        /usr/local/YT-HOME/YTHOME setting ${params}

        read -p "Do you want to change admin credentials [y/n]? ": admin_confirm
        if [[ "${admin_confirm}" == "y" || "${admin_confirm}" == "Y" ]]; then
            # First admin credentials
            read -p "Please set up your username:" config_account
            read -p "Please set up your password:" config_password

            # Set credentials
            echo -e "${yellow}Initializing, please wait...${plain}"
            /usr/local/YT-HOME/YTHOME admin -username "${config_account}" -password "${config_password}"
        else
            echo -e "${yellow}Your current admin credentials: ${plain}"
            /usr/local/YT-HOME/YTHOME admin -show
        fi
    else
        echo -e "${red}cancel...${plain}"
        if [[ "${fresh_install:-0}" -eq 1 ]]; then
            echo -e "this is a fresh installation, keeping the default login info:"
            echo -e "###############################################"
            /usr/local/YT-HOME/YTHOME admin -show
            echo -e "###############################################"
            echo -e "${red}if you forgot your login info,you can type ${green}YT-HOME${red} for configuration menu${plain}"
        else
            echo -e "${red} this is your upgrade,will keep old settings,if you forgot your login info,you can type ${green}YT-HOME${red} for configuration menu${plain}"
        fi
    fi
}

prepare_services() {
    if is_systemd && [[ -f "/etc/systemd/system/sing-box.service" ]]; then
        echo -e "${yellow}Stopping sing-box service... ${plain}"
        systemctl stop sing-box
        rm -f /usr/local/YT-HOME/bin/sing-box /usr/local/YT-HOME/bin/runSingbox.sh /usr/local/YT-HOME/bin/signal
    fi
    if [[ -e "/usr/local/YT-HOME/bin" ]]; then
        echo -e "###############################################################"
        echo -e "${green}/usr/local/YT-HOME/bin${red} directory exists yet!"
        echo -e "Please check the content and delete it manually after migration ${plain}"
        echo -e "###############################################################"
    fi
    if is_systemd; then
        systemctl daemon-reload
    fi
}

install_ythome() {
    cd /tmp/

    if [ $# == 0 ]; then
        last_version=$(curl -Ls "https://api.github.com/repos/${repo}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
        if [[ ! -n "$last_version" ]]; then
            echo -e "${red}Failed to fetch YT-HOME version, it maybe due to Github API restrictions, please try it later${plain}"
            exit 1
        fi
        echo -e "Got YT-HOME latest version: ${last_version}, beginning the installation..."
        wget -N --no-check-certificate -O /tmp/YT-HOME-linux-$(arch).tar.gz https://github.com/${repo}/releases/download/${last_version}/YT-HOME-linux-$(arch).tar.gz
        if [[ $? -ne 0 ]]; then
            echo -e "${red}Downloading YT-HOME failed, please be sure that your server can access Github ${plain}"
            exit 1
        fi
    else
        last_version=$1
        if [[ "${last_version}" != v* ]]; then
            last_version="v${last_version}"
        fi
        url="https://github.com/${repo}/releases/download/${last_version}/YT-HOME-linux-$(arch).tar.gz"
        echo -e "Beginning the install YT-HOME ${last_version}"
        wget -N --no-check-certificate -O /tmp/YT-HOME-linux-$(arch).tar.gz ${url}
        if [[ $? -ne 0 ]]; then
            echo -e "${red}download YT-HOME ${last_version} failed,please check the version exists${plain}"
            exit 1
        fi
    fi

    if [[ -e /usr/local/YT-HOME/ ]]; then
        if [[ -f /usr/local/YT-HOME/db/YT-HOME.db ]]; then
            fresh_install=0
        else
            fresh_install=1
        fi
        stop_service_if_exists
    else
        fresh_install=1
    fi

    tar zxvf YT-HOME-linux-$(arch).tar.gz
    rm YT-HOME-linux-$(arch).tar.gz -f

    chmod +x YT-HOME/YTHOME YT-HOME/YT-HOME.sh YT-HOME/sing-box
    cp YT-HOME/YT-HOME.sh /usr/bin/YT-HOME
    cp -rf YT-HOME /usr/local/
    install_service_files
    rm -rf YT-HOME

    config_after_install
    prepare_services

    enable_and_start_service

    echo -e "${green}YT-HOME v${last_version}${plain} installation finished, it is up and running now..."
    echo -e "You may access the Panel with following URL(s):${green}"
    /usr/local/YT-HOME/YTHOME uri
    echo -e "${plain}"
    echo -e ""
    YT-HOME help
}

echo -e "${green}Executing...${plain}"
install_base
install_ythome "$@"
