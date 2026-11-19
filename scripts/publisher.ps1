param([ValidateSet('Init','Bundle')][string]$Action = 'Bundle')
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $PSScriptRoot
$keyFolder = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'ConanForgePublishing'
$protectedKey = Join-Path $keyFolder 'updater-private-key.dpapi'
$publicKey = Join-Path $keyFolder 'updater-public.key'
$cli = Join-Path $project 'node_modules\.bin\tauri.cmd'
Add-Type -AssemblyName System.Security
Push-Location $project
try {
  if ($Action -eq 'Init') {
    if ((Test-Path -LiteralPath $protectedKey) -or (Test-Path -LiteralPath $publicKey)) {
      throw 'A publisher key already exists. Preserve it; do not generate a replacement.'
    }
    New-Item -ItemType Directory -Path $keyFolder -Force | Out-Null
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = Get-Acl -LiteralPath $keyFolder
    $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($identity,'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new([Security.Principal.SecurityIdentifier]::new('S-1-5-18'),'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
    Set-Acl -LiteralPath $keyFolder -AclObject $acl
    $temporaryKey = Join-Path $keyFolder 'new-updater.key'
    if (Test-Path -LiteralPath $temporaryKey) { throw 'A previous key initialization needs review.' }
    $output = & $cli signer generate --ci -w $temporaryKey 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'Key generation failed; inspect the private key folder.' }
    $plain = [IO.File]::ReadAllBytes($temporaryKey)
    $encrypted = [Security.Cryptography.ProtectedData]::Protect($plain,$null,'CurrentUser')
    $check = [Security.Cryptography.ProtectedData]::Unprotect($encrypted,$null,'CurrentUser')
    if ([Convert]::ToBase64String($plain) -ne [Convert]::ToBase64String($check)) { throw 'Key protection verification failed.' }
    [IO.File]::WriteAllBytes($protectedKey,$encrypted)
    Copy-Item -LiteralPath ($temporaryKey + '.pub') -Destination $publicKey
    Remove-Item -LiteralPath $temporaryKey
    Remove-Item -LiteralPath ($temporaryKey + '.pub')
    Copy-Item -LiteralPath $publicKey -Destination (Join-Path $project 'src-tauri\updater-public.key')
    Write-Output 'Publisher key created and protected for this Windows account. Only the public key was copied into the project.'
  } else {
    $config = Get-Content -LiteralPath src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    $configured = $config.plugins.updater.pubkey
    if ($configured.Trim() -ne [IO.File]::ReadAllText($publicKey).Trim()) { throw 'Publisher key does not match this application.' }
    $binary = Get-Item -LiteralPath (Join-Path $project 'src-tauri\target\release\conan-mod-manager.exe')
    if ($binary.VersionInfo.ProductVersion -ne $config.version) { throw 'Build the current app version before bundling.' }
    $encrypted = [IO.File]::ReadAllBytes($protectedKey)
    $plain = [Security.Cryptography.ProtectedData]::Unprotect($encrypted,$null,'CurrentUser')
    $env:TAURI_SIGNING_PRIVATE_KEY = [Text.Encoding]::UTF8.GetString($plain)
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
    & $cli bundle --bundles nsis --config src-tauri/tauri.installer.conf.json
    if ($LASTEXITCODE -ne 0) { throw 'Signed installer bundling failed.' }
  }
} finally {
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  if ($plain) { [Array]::Clear($plain,0,$plain.Length) }
  if ($check) { [Array]::Clear($check,0,$check.Length) }
  Pop-Location
}
