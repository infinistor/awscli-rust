# Portal·Mover·ZeroMQ 기준 출력(`tests/parity/baseline/{portal,mover,zeromq}`)을 TESTCore로 다시 만든다.
#
#   pwsh tests/parity/gen-client-baselines.ps1 [-TestCoreBin E:\Code\Git\TESTCore\bin\TestCore]
#
# 사례(`tests/parity/{portal,mover,zeromq}/*.json`)마다 오라클의 같은 이름 명령을 실행해 결과를 저장한다.
# 포트와 연결 시각 같은 실행마다 달라지는 값은 저장하지 않고, 비교할 때 테스트가 처리한다.
param([string]$TestCoreBin = "E:\Code\Git\TESTCore\bin\TestCore")

$ErrorActionPreference = 'Stop'
$repo = Resolve-Path (Join-Path $PSScriptRoot '..\..')
dotnet build (Join-Path $repo 'tools/dotnet-oracle') "-p:TestCoreBin=$TestCoreBin" | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'oracle build failed' }
$env:TESTCORE_BIN = $TestCoreBin
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$oracle = Join-Path $repo 'tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll'

foreach ($kind in 'portal', 'mover', 'zeromq') {
  if (-not (Test-Path (Join-Path $PSScriptRoot $kind))) { continue }
  New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot "baseline/$kind") | Out-Null
  foreach ($case in Get-ChildItem (Join-Path $PSScriptRoot $kind) -Filter *.json) {
    $out = Join-Path $PSScriptRoot "baseline/$kind/$($case.Name)"
    $text = (dotnet $oracle $kind $case.FullName) -join "`n"
    [IO.File]::WriteAllText($out, $text + "`n", (New-Object Text.UTF8Encoding $false))
    "$kind/$($case.Name)"
  }
}
