# BPMNCode Windows Installer
param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

function Write-Info($Message) {
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Error($Message) {
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/BPMNCode/BPMNCode/releases/latest"
        return $response.tag_name
    } catch {
        Write-Error "Failed to fetch latest version"
    }
}

function Install-BPMNCode {
    Write-Info "Starting BPMNCode installation..."
    
    # Определяем архитектуру
    $arch = if ([Environment]::Is64BitOperatingSystem) { "amd64" } else { "386" }
    
    # Получаем версию
    if ($Version -eq "latest") {
        $Version = Get-LatestVersion
    }
    
    Write-Info "Installing BPMNCode $Version for Windows-$arch"
    
    # URL для скачивания
    $filename = "bpmncode-windows-$arch.zip"
    $url = "https://github.com/BPMNCode/BPMNCode/releases/download/$Version/$filename"
    
    # Создаем временную директорию
    $tempDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    
    try {
        # Скачиваем файл
        Write-Info "Downloading from $url..."
        $zipPath = Join-Path $tempDir $filename
        Invoke-WebRequest -Uri $url -OutFile $zipPath
        
        # Извлекаем архив
        Write-Info "Extracting archive..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir
        
        # Устанавливаем бинарник
        Write-Info "Installing binary..."
        $installDir = "$env:USERPROFILE\bin"
        if (!(Test-Path $installDir)) {
            New-Item -ItemType Directory -Path $installDir | Out-Null
        }
        
        Copy-Item -Path (Join-Path $tempDir "bpmncode.exe") -Destination $installDir
        
        # Добавляем в PATH если не добавлено
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if ($userPath -notlike "*$installDir*") {
            [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
            Write-Info "Added $installDir to PATH"
        }
        
        Write-Info "BPMNCode successfully installed!"
        Write-Info "Restart your terminal and try: bpmncode --help"
        
    } finally {
        # Очищаем временные файлы
        Remove-Item -Path $tempDir -Recurse -Force
    }
}

Install-BPMNCode