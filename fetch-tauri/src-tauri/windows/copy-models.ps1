# PowerShell script to copy models folder from MSI directory to installation directory
param(
    [string]$CustomActionData
)

# Parse CustomActionData (format: "MSIDIR=value;INSTALLDIR=value")
$properties = @{}
$CustomActionData -split ';' | ForEach-Object {
    if ($_ -match '(.+)=(.+)') {
        $properties[$matches[1]] = $matches[2]
    }
}

$msiDir = $properties['MSIDIR']
$installDir = $properties['INSTALLDIR']

# Log to a file for debugging
$logFile = Join-Path $env:TEMP "fetch-models-copy.log"
"MSI Directory: $msiDir" | Out-File -FilePath $logFile -Append
"Install Directory: $installDir" | Out-File -FilePath $logFile -Append

# Get the parent directory of the MSI file (where the models folder should be)
$msiParentDir = Split-Path -Parent $msiDir
$modelsSource = Join-Path $msiParentDir "models"
$modelsTarget = Join-Path $installDir "models"

"Models Source: $modelsSource" | Out-File -FilePath $logFile -Append
"Models Target: $modelsTarget" | Out-File -FilePath $logFile -Append

# Check if source models folder exists
if (Test-Path $modelsSource) {
    "Source folder exists" | Out-File -FilePath $logFile -Append

    # Remove target if it exists
    if (Test-Path $modelsTarget) {
        "Removing existing target folder" | Out-File -FilePath $logFile -Append
        Remove-Item -Path $modelsTarget -Recurse -Force
    }

    # Copy the models folder
    "Copying models folder..." | Out-File -FilePath $logFile -Append
    Copy-Item -Path $modelsSource -Destination $modelsTarget -Recurse -Force

    "Copy completed successfully" | Out-File -FilePath $logFile -Append
    exit 0
} else {
    "ERROR: Source folder not found: $modelsSource" | Out-File -FilePath $logFile -Append
    # Don't fail the installation, just log the error
    exit 0
}
