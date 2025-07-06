#!/bin/bash
set -e

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функция для логирования
log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Определяем OS
detect_os() {
    case "$OSTYPE" in
        darwin*)  echo "macos" ;;
        linux*)   echo "linux" ;;
        msys*)    echo "windows" ;;
        cygwin*)  echo "windows" ;;
        *)        error "Unsupported OS: $OSTYPE" ;;
    esac
}

# Определяем архитектуру
detect_arch() {
    case $(uname -m) in
        x86_64)  echo "amd64" ;;
        aarch64) echo "arm64" ;;
        arm64)   echo "arm64" ;;
        *)       error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Основная функция установки
install_bpmncode() {
    log "Starting BPMNCode installation..."
    
    # Определяем параметры системы
    OS=$(detect_os)
    ARCH=$(detect_arch)
    
    # Версия (можно передать как аргумент)
    VERSION=${1:-"latest"}
    
    # Если latest, получаем последнюю версию через GitHub API
    if [ "$VERSION" = "latest" ]; then
        log "Fetching latest version..."
        VERSION=$(curl -s https://api.github.com/repos/BPMNCode/BPMNCode/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
        if [ -z "$VERSION" ]; then
            error "Failed to fetch latest version"
        fi
    fi
    
    log "Installing BPMNCode $VERSION for $OS-$ARCH"
    
    # Формируем URL для скачивания
    if [ "$OS" = "windows" ]; then
        FILENAME="bpmncode-windows-${ARCH}.zip"
    else
        FILENAME="bpmncode-${OS}-${ARCH}.tar.gz"
    fi
    
    URL="https://github.com/BPMNCode/BPMNCode/releases/download/$VERSION/$FILENAME"
    
    # Создаем временную директорию
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"
    
    # Скачиваем файл
    log "Downloading from $URL..."
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$FILENAME" "$URL" || error "Failed to download"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$FILENAME" "$URL" || error "Failed to download"
    else
        error "Neither curl nor wget is available"
    fi
    
    # Извлекаем архив
    log "Extracting archive..."
    if [ "$OS" = "windows" ]; then
        unzip "$FILENAME" || error "Failed to extract"
        BINARY_NAME="bpmncode.exe"
    else
        tar -xzf "$FILENAME" || error "Failed to extract"
        BINARY_NAME="bpmncode"
    fi
    
    # Устанавливаем бинарник
    log "Installing binary..."
    if [ "$OS" = "windows" ]; then
        # На Windows устанавливаем в PATH или текущую директорию
        INSTALL_DIR="$HOME/bin"
        mkdir -p "$INSTALL_DIR"
        cp "$BINARY_NAME" "$INSTALL_DIR/"
        warn "Add $INSTALL_DIR to your PATH to use bpmncode globally"
    else
        # На Unix системах устанавливаем в /usr/local/bin
        if [ -w "/usr/local/bin" ]; then
            cp "$BINARY_NAME" "/usr/local/bin/"
            chmod +x "/usr/local/bin/$BINARY_NAME"
        else
            log "Installing to /usr/local/bin (requires sudo)..."
            sudo cp "$BINARY_NAME" "/usr/local/bin/"
            sudo chmod +x "/usr/local/bin/$BINARY_NAME"
        fi
    fi
    
    # Очищаем временные файлы
    cd - >/dev/null
    rm -rf "$TEMP_DIR"
    
    # Проверяем установку
    log "Verifying installation..."
    if command -v bpmncode >/dev/null 2>&1; then
        VERSION_OUTPUT=$(bpmncode --version 2>/dev/null || echo "unknown")
        log "BPMNCode successfully installed! Version: $VERSION_OUTPUT"
        log "Try: bpmncode --help"
    else
        error "Installation failed - bpmncode not found in PATH"
    fi
}

# Проверяем зависимости
check_dependencies() {
    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required but not installed"
    fi
    
    if [ "$OS" = "windows" ] && ! command -v unzip >/dev/null 2>&1; then
        error "unzip is required but not installed"
    fi
}

# Главная функция
main() {
    echo "BPMNCode Installer"
    echo "=================="
    
    check_dependencies
    install_bpmncode "$@"
}

# Запускаем если скрипт вызван напрямую
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi

main "$@"
