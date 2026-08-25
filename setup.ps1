# Put `polite` on the PATH, so it can be typed from anywhere.
#
#     .\setup.ps1              build it, then register the command
#     .\setup.ps1 -SkipBuild   register the command only
#     .\setup.ps1 -Remove      take it off the PATH again
#
# Only the PATH of the person running it is touched. Nothing is written anywhere else on the
# machine, nothing is installed, and the whole of it can be undone with -Remove.

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$Remove
)

$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$dir = Join-Path $here 'target\release'
$backup = Join-Path $HOME '.polite-path-backup.txt'

function Get-UserPathRaw {
    # Read it without expanding, so that an entry written as %USERPROFILE%\... comes back the way
    # it was written rather than as the place it happens to point at today.
    (Get-Item 'HKCU:\Environment').GetValue('Path', '', 'DoNotExpandEnvironmentNames')
}

function Set-UserPathRaw($value) {
    # ExpandString on purpose. Writing the PATH back as a plain string is the usual way this job
    # goes wrong: every %USERPROFILE% in it stops being a variable and becomes thirteen literal
    # characters, and the entries using it quietly stop working.
    Set-ItemProperty -Path 'HKCU:\Environment' -Name 'Path' -Value $value -Type ExpandString
}

function Announce-Change {
    # Windows Explorer keeps its own copy of the environment and hands it to everything it starts,
    # so without this a new terminal would not see the change until the next time you signed in.
    Add-Type -Namespace Win32 -Name PoliteEnv -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@ -ErrorAction SilentlyContinue
    $result = [UIntPtr]::Zero
    $null = [Win32.PoliteEnv]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
}

if ($Remove) {
    $raw = Get-UserPathRaw
    $kept = @($raw -split ';' | Where-Object { $_ -ne '' -and $_ -ne $dir })
    if ($kept.Count -eq ($raw -split ';' | Where-Object { $_ -ne '' }).Count) {
        Write-Host "  polite was not on your PATH, so there was nothing to take off."
    } else {
        Set-UserPathRaw ($kept -join ';')
        Announce-Change
        Write-Host "  Taken off your PATH. Open a new terminal for it to stop being found."
    }
    return
}

if (-not $SkipBuild) {
    Write-Host "  Building. On a slow machine this takes a couple of minutes."
    Push-Location $here
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "the build did not finish" }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path (Join-Path $dir 'polite.exe'))) {
    throw "I cannot find polite.exe in $dir. Run this without -SkipBuild to build it first."
}

$raw = Get-UserPathRaw
if (-not (Test-Path $backup)) {
    Set-Content -Path $backup -Value $raw -Encoding utf8
    Write-Host "  Your PATH as it was is kept in $backup"
}

if (($raw -split ';') -contains $dir) {
    Write-Host "  polite is already on your PATH. Nothing to do."
} else {
    Set-UserPathRaw (($raw.TrimEnd(';')) + ';' + $dir)
    Announce-Change
    Write-Host "  Added $dir to your PATH."
}

Write-Host ""
Write-Host "  Open a new terminal, then try:"
Write-Host ""
Write-Host "      polite words --tier everyday"
Write-Host "      polite run `"$here\examples\guide\01-hello.polite`""
Write-Host ""
