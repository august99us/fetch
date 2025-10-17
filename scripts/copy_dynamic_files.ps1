# Script to copy ONNX runtime libraries and models to target directory
# Env variable $ONNX_BUILD_PATH must be set to the directory containing the lib files or dylibs will not be copied
# Usage: .\copy_dynamic_files.ps1 <profile> <use_symlinks>
# Arguments:
#   profile: "debug" or "release"
#   use_symlinks: $true to create symlinks, $false to copy files

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("debug", "release")]
    [string]$Profile,
    
    [Parameter(Mandatory=$true)]
    [bool]$UseSymlinks
)

# Get script directory and workspace root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$WorkspaceRoot = Split-Path -Parent $ScriptDir

# Determine target directory
$TargetDir = Join-Path $WorkspaceRoot "target\$Profile"
Write-Host "Target directory: $TargetDir"

# Create target directory if it doesn't exist
if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

# Function to copy or symlink file/directory
function Copy-OrLink {
    param(
        [string]$Source,
        [string]$Destination
    )
    
    $Name = Split-Path -Leaf $Source
    
    if (-not (Test-Path $Source)) {
        Write-Warning "$Name does not exist: $Source"
        return
    }
    
    $DestPath = Join-Path $Destination $Name
    
    # Remove existing file/symlink/directory
    if (Test-Path $DestPath) {
        Remove-Item $DestPath -Recurse -Force
    }
    
    if ($UseSymlinks) {
        # Create link (junction for directories, hard link for files to avoid admin privileges)
        $AbsSource = Resolve-Path $Source
        if (Test-Path $Source -PathType Container) {
            # Directory junction (no admin required)
            try {
                New-Item -ItemType Junction -Path $DestPath -Target $AbsSource | Out-Null
                Write-Host "Created directory junction: $DestPath -> $AbsSource"
            } catch {
                Write-Warning "Failed to create junction, falling back to copy: $_"
                Copy-Item $Source $DestPath -Recurse -Force
                Write-Host "Copied directory: $Source -> $DestPath"
            }
        } else {
            # Try hard link first (no admin required), fall back to symbolic link, then copy
            try {
                New-Item -ItemType HardLink -Path $DestPath -Target $AbsSource | Out-Null
                Write-Host "Created hard link: $DestPath -> $AbsSource"
            } catch {
                try {
                    New-Item -ItemType SymbolicLink -Path $DestPath -Target $AbsSource | Out-Null
                    Write-Host "Created symbolic link: $DestPath -> $AbsSource"
                } catch {
                    Write-Warning "Failed to create hard/symbolic link, falling back to copy: $_"
                    Copy-Item $Source $DestPath -Force
                    Write-Host "Copied file: $Source -> $DestPath"
                }
            }
        }
    } else {
        # Copy file/directory
        if (Test-Path $Source -PathType Container) {
            Copy-Item $Source $DestPath -Recurse -Force
            Write-Host "Copied directory: $Source -> $DestPath"
        } else {
            Copy-Item $Source $DestPath -Force
            Write-Host "Copied file: $Source -> $DestPath"
        }
    }
}

# Copy models from fetch-core/artifacts
$ModelsSource = Join-Path $WorkspaceRoot "fetch-core\bundle\models"
if (Test-Path $ModelsSource) {
    Copy-OrLink -Source $ModelsSource -Destination $TargetDir
} else {
    Write-Warning "Models directory not found: $ModelsSource"
}

# Copy ONNX runtime libraries
$OnnxBuildPath = $env:ONNX_BUILD_PATH
if ($OnnxBuildPath) {
    Write-Host "ONNX build path: $OnnxBuildPath"
    
    if (-not (Test-Path $OnnxBuildPath)) {
        Write-Error "ONNX build path does not exist: $OnnxBuildPath"
        exit 1
    }
    
    # Define DLLs based on platform
    $IsWindows = $env:OS -eq "Windows_NT"
    $IsMacOS = $env:OSTYPE -like "darwin*"
    
    if ($IsWindows -or $env:OSTYPE -like "*msys*" -or $env:OSTYPE -eq "win32") {
        # Windows DLLs
        $OnnxLibs = @(
            "onnxruntime.dll",
            "onnxruntime_providers_shared.dll",
            "onnxruntime_providers_qnn.dll",
            "QnnHtp.dll",
            "QnnSystem.dll",
            "onnxruntime_providers_cuda.dll",
            "cudart64_12.dll",
            "cublasLt64_12.dll",
            "cublas64_12.dll"
        )
    } elseif ($IsMacOS) {
        # macOS dylibs
        $OnnxLibs = @(
            "libonnxruntime.dylib",
            "libonnxruntime_providers_shared.dylib",
            "libonnxruntime_providers_qnn.dylib",
            "libQnnHtp.dylib",
            "libQnnSystem.dylib",
            "libonnxruntime_providers_cuda.dylib"
        )
    } else {
        # Linux .so files
        $OnnxLibs = @(
            "libonnxruntime.so",
            "libonnxruntime_providers_shared.so",
            "libonnxruntime_providers_qnn.so",
            "libQnnHtp.so",
            "libQnnSystem.so",
            "libonnxruntime_providers_cuda.so"
        )
    }
    
    # Copy each library
    foreach ($Lib in $OnnxLibs) {
        $LibPath = Join-Path $OnnxBuildPath $Lib
        if (Test-Path $LibPath) {
            Copy-OrLink -Source $LibPath -Destination $TargetDir
        }
    }
} else {
    Write-Warning "ONNX_BUILD_PATH environment variable not set, skipping ONNX library copy"
}

Write-Host "Done copying libraries and models to $TargetDir"