$OutputFolder = ".\RandomFiles"

if (!(Test-Path $OutputFolder)) {
    New-Item -ItemType Directory -Path $OutputFolder | Out-Null
}

$startSize = 1GB
$endSize = 5GB
$step = 500MB

for ($size = $startSize; $size -le $endSize; $size += $step) {
    $randomName = [System.IO.Path]::GetRandomFileName() + ".bin"
    $filePath = Join-Path $OutputFolder $randomName
    fsutil file createnew $filePath $size | Out-Null
    Write-Host "Created $randomName with size $([math]::Round($size/1MB)) MB"
}

Write-Host "Done!"
