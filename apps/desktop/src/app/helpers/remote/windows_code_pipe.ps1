param([Parameter(Mandatory = $true)][string]$PipePath)
$ErrorActionPreference = 'Stop'

$gxPipePrefix = '\\.\pipe\ghostex-code-'
if (!$PipePath.StartsWith($gxPipePrefix, [StringComparison]::Ordinal) -or
    $PipePath.Substring($gxPipePrefix.Length) -cnotmatch '^[0-9a-f]{64}$') {
    throw 'Invalid Ghostex Code IPC pipe.'
}

$gxPipeUserSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$gxPipeSecurity = [IO.Pipes.PipeSecurity]::new()
$gxPipeSecurity.SetSecurityDescriptorSddlForm("D:P(A;;GA;;;SY)(A;;GA;;;$gxPipeUserSid)", [Security.AccessControl.AccessControlSections]::Access)
$gxPipe = [IO.Pipes.NamedPipeClientStream]::new('.', $PipePath.Substring(9),
    ([IO.Pipes.PipeAccessRights]::ReadWrite -bor [IO.Pipes.PipeAccessRights]::ChangePermissions), [IO.Pipes.PipeOptions]::None,
    [Security.Principal.TokenImpersonationLevel]::None, [IO.HandleInheritability]::None)
try {
    $gxPipe.Connect(10000)
    $gxPipe.SetAccessControl($gxPipeSecurity)
} finally {
    $gxPipe.Dispose()
}
