# Prepare the toolchain the native Windows editor build (VS Code REH for win32,
# apps/desktop/scripts/build-windows-code-server.ps1) needs on a GitHub Actions
# Windows runner: Python, jq and git-lfs on PATH, the Visual Studio C++ tools with
# the Spectre-mitigated runtimes for the target architecture, and the pinned
# VS Code source tree. Exports PYTHON, npm_config_msvs_version and vs2022_install
# to GITHUB_ENV for the steps that follow. Shared by release-gpui-windows.yml and
# release-gpui-code-server-windows.yml so both keep one definition.
param(
    [Parameter(Mandatory = $true)][ValidateSet("x64", "arm64")][string]$Arch
)

$ErrorActionPreference = 'Stop'
if (-not $env:GITHUB_ENV) { throw 'GITHUB_ENV is required; this helper runs inside a GitHub Actions job.' }

foreach ($tool in @('python', 'jq', 'git-lfs')) {
    Get-Command $tool -ErrorAction Stop | Out-Null
}
"PYTHON=$((Get-Command python).Source)" >> $env:GITHUB_ENV
$vswhere = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/vswhere.exe"
$vs = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (!$vs) { throw 'The native editor requires Visual Studio C++ build tools.' }
$component = if ($Arch -eq 'arm64') {
    'Microsoft.VisualStudio.Component.VC.Runtimes.ARM64.Spectre'
} else {
    'Microsoft.VisualStudio.Component.VC.Runtimes.x86.x64.Spectre'
}
$installed = & $vswhere -latest -products '*' -requires $component -property installationPath
if (!$installed -or $installed.Trim() -ne $vs) {
    $installer = "${env:ProgramFiles(x86)}/Microsoft Visual Studio/Installer/setup.exe"
    $install = Start-Process $installer -Wait -PassThru -ArgumentList @('modify', '--installPath', ('"' + $vs + '"'), '--add', $component, '--quiet', '--norestart')
    if ($install.ExitCode -notin @(0, 3010)) { throw "Spectre libraries installation failed: $($install.ExitCode)" }
}
"npm_config_msvs_version=$vs" >> $env:GITHUB_ENV
# CDXC:CodeEditor 2026-09-14 WHY:
# The release runners ship Visual Studio 2026, and VS Code's preinstall probe only recognizes the 2019 and 2022 install layouts,
# so it rejected the toolchain and the native editor build failed before it started. Point its documented override at the
# installation vswhere already resolved above.
"vs2022_install=$vs" >> $env:GITHUB_ENV
git -C .dependencies/code-server submodule update --init --depth=1 -- lib/vscode
if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the pinned VS Code sources.' }
