#!/usr/bin/env pwsh

# Proton Pass CLI Installation Script for Windows
# Usage: Invoke-WebRequest -Uri https://proton.me/download/pass-cli/install.ps1 -OutFile install.ps1; .\install.ps1
# Or with custom install dir: $env:PROTON_PASS_CLI_INSTALL_DIR="C:\custom\path"; .\install.ps1
# Or with custom channel: $env:PROTON_PASS_CLI_INSTALL_CHANNEL="beta"; .\install.ps1

$ErrorActionPreference = "Stop"

$MANIFEST_BASE_URL = "https://proton.me/download/pass-cli/"
$BINARY_NAME = "pass-cli.exe"

# Get manifest URL based on channel
function Get-ManifestUrl {
    $channel = $env:PROTON_PASS_CLI_INSTALL_CHANNEL
    if ($null -eq $channel) {
        $channel = ""
    }
    $channel = $channel.Trim()
    
    if ([string]::IsNullOrEmpty($channel) -or $channel -eq "stable") {
        return "${MANIFEST_BASE_URL}versions.json"
    }
    else {
        return "${MANIFEST_BASE_URL}versions.${channel}.json"
    }
}

$MANIFEST_URL = Get-ManifestUrl

# Color output functions
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# Detect architecture
function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "x86_64" { return "x86_64" }
        default {
            Write-Error-Custom "Unsupported architecture: $arch"
            Write-Error-Custom "Only x86_64 (64-bit) is supported on Windows"
            exit 1
        }
    }
}

# Fetch and parse manifest
function Get-BinaryInfo {
    param([string]$Arch)
    
    Write-Info "Fetching manifest from $MANIFEST_URL..."
    
    try {
        $response = Invoke-WebRequest -Uri $MANIFEST_URL -UseBasicParsing
        $manifest = $response.Content | ConvertFrom-Json
    }
    catch {
        Write-Error-Custom "Failed to download manifest from $MANIFEST_URL"
        Write-Error-Custom $_.Exception.Message
        exit 1
    }
    
    # Check format version
    $formatVer = $manifest.formatVersion
    if ($null -eq $formatVer -or $formatVer -ne 1) {
        Write-Error-Custom "Unsupported manifest format version: $formatVer"
        Write-Error-Custom "Please upgrade your installation script or install manually."
        Write-Error-Custom "Manifest content preview: $($manifest | ConvertTo-Json -Depth 1)"
        exit 1
    }
    
    # Extract version
    $version = $manifest.passCliVersions.version
    
    if ([string]::IsNullOrEmpty($version)) {
        Write-Error-Custom "Could not determine latest version from manifest"
        exit 1
    }
    
    Write-Info "Latest version: $version"
    
    # Extract URL and hash for Windows platform
    $binaryInfo = $manifest.passCliVersions.urls.windows.$Arch
    
    if ($null -eq $binaryInfo) {
        Write-Error-Custom "No binary available for platform: windows/$Arch"
        exit 1
    }
    
    $url = $binaryInfo.url
    $hash = $binaryInfo.hash
    
    if ([string]::IsNullOrEmpty($url) -or [string]::IsNullOrEmpty($hash)) {
        Write-Error-Custom "Invalid binary information in manifest"
        exit 1
    }
    
    return @{
        Version = $version
        Url = $url
        Hash = $hash
    }
}

# Download and verify binary
function Download-Binary {
    param(
        [string]$Url,
        [string]$ExpectedHash,
        [string]$TempFile
    )
    
    Write-Info "Downloading binary from $Url..."
    
    try {
        Invoke-WebRequest -Uri $Url -OutFile $TempFile -UseBasicParsing
    }
    catch {
        Write-Error-Custom "Failed to download binary from $Url"
        Write-Error-Custom $_.Exception.Message
        exit 1
    }
    
    Write-Info "Verifying SHA256 hash..."
    
    try {
        $fileHash = Get-FileHash -Path $TempFile -Algorithm SHA256
        $computedHash = $fileHash.Hash.ToLower()
    }
    catch {
        Write-Error-Custom "Failed to compute file hash"
        Write-Error-Custom $_.Exception.Message
        Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
        exit 1
    }
    
    if ($computedHash -ne $ExpectedHash.ToLower()) {
        Write-Error-Custom "Hash verification failed!"
        Write-Error-Custom "Expected: $ExpectedHash"
        Write-Error-Custom "Got:      $computedHash"
        Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
        exit 1
    }
    
    Write-Info "Hash verification successful"

    Write-Info "Extracting zip file..."
    $extractDir = Join-Path ([System.IO.Path]::GetTempPath()) ("pass-cli-extract-" + [System.Guid]::NewGuid().ToString())

    try {
        Expand-Archive -Path $TempFile -DestinationPath $extractDir -Force
        Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
        return $extractDir
    }
    catch {
        Write-Error-Custom "Failed to extract zip file"
        Write-Error-Custom $_.Exception.Message
        Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
        Remove-Item -Path $extractDir -Recurse -Force -ErrorAction SilentlyContinue
        exit 1
    }
}

# Get install directory
function Get-InstallDir {
    if ($env:PROTON_PASS_CLI_INSTALL_DIR) {
        return $env:PROTON_PASS_CLI_INSTALL_DIR
    }
    
    # Default to user's local programs directory
    $localAppData = [System.Environment]::GetFolderPath('LocalApplicationData')
    return Join-Path $localAppData "Programs\ProtonPass"
}

# Install binary
function Install-Binary {
    param([string]$TempFileOrDir)
    
    $installDir = Get-InstallDir
    
    Write-Info "Installing to $installDir..."
    
    # Create install directory if it doesn't exist
    if (-not (Test-Path $installDir)) {
        try {
            New-Item -ItemType Directory -Path $installDir -Force | Out-Null
        }
        catch {
            Write-Error-Custom "Failed to create install directory: $installDir"
            Write-Error-Custom $_.Exception.Message
            exit 1
        }
    }
    
    # Check if it's a directory (extracted zip) or a file
    if (Test-Path $TempFileOrDir -PathType Container) {
        # Copy all files from extracted directory
        try {
            $files = Get-ChildItem -Path $TempFileOrDir -File
            foreach ($file in $files) {
                $targetPath = Join-Path $installDir $file.Name
                Copy-Item -Path $file.FullName -Destination $targetPath -Force
            }
        }
        catch {
            Write-Error-Custom "Failed to install files to $installDir"
            Write-Error-Custom $_.Exception.Message
            exit 1
        }
        
        # Clean up temp directory
        Remove-Item -Path $TempFileOrDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    else {
        Write-Error-Custom "Downloaded an invalid file"
        exit 1
    }

    Write-Info "Installation complete!"
    Write-Host ""

    # Set release track if custom channel was used during installation
    $channel = $env:PROTON_PASS_CLI_INSTALL_CHANNEL
    if ($null -ne $channel) {
        $channel = $channel.Trim()
    }

    if (-not [string]::IsNullOrEmpty($channel) -and $channel -ne "stable") {
        Write-Info "Setting release track to $channel..."
        try {
            $output = & $targetPath update --set-track $channel 2>&1
            Write-Info "Release track set successfully"
        }
        catch {
            Write-Warn "Could not set release track automatically. You can set it manually later with: $BINARY_NAME update --set-track $channel"
        }
        Write-Host ""
    }

    # Check if install dir is in PATH
    $pathDirs = $env:PATH -split ';'
    $inPath = $pathDirs -contains $installDir

    if (-not $inPath) {
        Write-Warn "Installation directory is not in your PATH"
        Write-Host ""
        Write-Host "To use $BINARY_NAME from anywhere, add the installation directory to your PATH:"
        Write-Host ""
        Write-Host "Run this command in PowerShell (as Administrator):"
        Write-Host ""
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$installDir', 'User')"
        Write-Host ""
        Write-Host "Or manually add this directory to your PATH:"
        Write-Host "  $installDir"
        Write-Host ""
        Write-Host "After updating PATH, restart your terminal."
        Write-Host ""
    }
    else {
        Write-Host "You can now run: $BINARY_NAME --help"
    }
}

# Main installation flow
function Main {
    Write-Info "Starting Proton Pass CLI installation..."
    Write-Host ""
    
    # Detect architecture
    $arch = Get-Architecture
    Write-Info "Detected architecture: $arch"
    
    # Fetch binary info from manifest
    $binaryInfo = Get-BinaryInfo -Arch $arch
    
    # Determine file extension from URL
    $isZip = $binaryInfo.Url -like "*.zip"
    
    # Download binary to temp file
    $tempFile = [System.IO.Path]::GetTempFileName()
    if ($isZip) {
        $tempDownload = [System.IO.Path]::ChangeExtension($tempFile, ".zip")
    }
    else {
        Write-Error-Custom "Downloaded an invalid file, it should be a zip file"
        exit 1
    }
    Move-Item -Path $tempFile -Destination $tempDownload -Force
    
    try {
        $extractedDir = Download-Binary -Url $binaryInfo.Url -ExpectedHash $binaryInfo.Hash -TempFile $tempDownload
        
        # Install binary (pass directory of the extracted zip)
        Install-Binary -TempFileOrDir $extractedDir
    }
    finally {
        # Clean up temp file if it still exists
        if (Test-Path $tempDownload) {
            Remove-Item -Path $tempDownload -Force -ErrorAction SilentlyContinue
        }
    }
}

# Run main function
Main


