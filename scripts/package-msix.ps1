param(
    [Parameter(Mandatory)][string]$IdentityName,
    [Parameter(Mandatory)][string]$Publisher,
    [Parameter(Mandatory)][string]$PublisherDisplayName,
    [string]$AppDirectory = "artifacts/installers/win-unpacked"
)
$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot ".."
$source = Join-Path $root $AppDirectory
$metadata = Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json
if ($metadata.version -notmatch '^\d+\.\d+\.\d+$') {
    throw "MSIX requires a release version in major.minor.patch format."
}
if ($IdentityName -notmatch '^[A-Za-z0-9.-]{3,50}$' -or $Publisher -notmatch '^CN=') {
    throw "Use the package Name and Publisher from Partner Center."
}
$executable = 'MemoIvy.exe'
foreach ($file in @($executable, 'resources\engine\memoivy-engine.exe', 'resources\app.asar')) {
    if (!(Test-Path (Join-Path $source $file) -PathType Leaf)) { throw "Missing package input: $file" }
}
$kit = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (!$kit) { throw "Install the Windows SDK with MakeAppx." }
$staging = New-Item -ItemType Directory -Path (Join-Path ([IO.Path]::GetTempPath()) ("memoivy-msix-" + [guid]::NewGuid()))
try {
    Copy-Item (Join-Path $source '*') $staging.FullName -Recurse
    $assets = New-Item -ItemType Directory -Path (Join-Path $staging.FullName 'Assets')
    Add-Type -AssemblyName System.Drawing
    $image = [Drawing.Image]::FromFile((Join-Path $root 'desktop/icon.png'))
    try {
        foreach ($size in @(44, 50, 150)) {
            $bitmap = [Drawing.Bitmap]::new($size, $size)
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.DrawImage($image, 0, 0, $size, $size)
                $bitmap.Save((Join-Path $assets.FullName "Logo$size.png"), [Drawing.Imaging.ImageFormat]::Png)
            } finally {
                $graphics.Dispose()
                $bitmap.Dispose()
            }
        }
    } finally {
        $image.Dispose()
    }

    [xml]$manifest = Get-Content (Join-Path $root 'desktop/AppxManifest.xml') -Raw
    $manifest.Package.Identity.Name = $IdentityName
    $manifest.Package.Identity.Publisher = $Publisher
    $manifest.Package.Identity.Version = "$($metadata.version).0"
    $manifest.Package.Properties.PublisherDisplayName = $PublisherDisplayName
    $manifest.Save((Join-Path $staging.FullName 'AppxManifest.xml'))

    $output = Join-Path $root "artifacts/installers/MemoIvy-$($metadata.version)-x64.msix"
    & $kit.FullName pack /d $staging.FullName /p $output /o
    if ($LASTEXITCODE) { throw "MakeAppx failed ($LASTEXITCODE)." }
} finally {
    Remove-Item $staging.FullName -Recurse -Force
}
