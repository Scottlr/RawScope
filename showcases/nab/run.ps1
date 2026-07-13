[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$workbenchFileName = if ([System.Environment]::OSVersion.Platform -eq 'Win32NT') {
    'rawscope-workbench.exe'
} else {
    'rawscope-workbench'
}
$releaseDirectory = Join-Path (Join-Path $repositoryRoot 'target') 'release'
$workbenchPath = Join-Path $releaseDirectory $workbenchFileName
$previousWorkbench = [System.Environment]::GetEnvironmentVariable('RAWSCOPE_WORKBENCH', 'Process')

Push-Location $repositoryRoot
try {
    cargo build --release -p rawscope-workbench
    if ($LASTEXITCODE -ne 0) {
        throw "RawScope workbench build exited with code $LASTEXITCODE"
    }

    [System.Environment]::SetEnvironmentVariable('RAWSCOPE_WORKBENCH', $workbenchPath, 'Process')
    cargo run --release -p rawscope-showcase-nab -- run
    if ($LASTEXITCODE -ne 0) {
        throw "NAB showcase exited with code $LASTEXITCODE"
    }
} finally {
    [System.Environment]::SetEnvironmentVariable(
        'RAWSCOPE_WORKBENCH',
        $previousWorkbench,
        'Process'
    )
    Pop-Location
}
