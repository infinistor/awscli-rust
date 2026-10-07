# UpDownClient parity 사례(tests/parity/updown/*.json)와 .NET 기준 출력(tests/parity/baseline/updown/*.json)을 만든다.
# 저장소 루트에서 실행한다: pwsh tools/dotnet-oracle/gen-updown-cases.ps1
$ErrorActionPreference = "Stop"
$d = "tests/parity/updown"
New-Item -ItemType Directory -Force $d, tests/parity/baseline/updown | Out-Null
$ns = 'xmlns="http://s3.amazonaws.com/doc/2006-03-01/"'
$listV2 = "<ListBucketResult $ns><Name>my-bucket</Name><KeyCount>2</KeyCount><IsTruncated>false</IsTruncated><Contents><Key>TH_001/k1</Key><Size>11</Size></Contents><Contents><Key>TH_001/k2</Key><Size>11</Size></Contents></ListBucketResult>"
$listV2Empty = "<ListBucketResult $ns><Name>my-bucket</Name><KeyCount>0</KeyCount><IsTruncated>false</IsTruncated></ListBucketResult>"
$listV1Trunc = "<ListBucketResult $ns><Name>my-bucket</Name><IsTruncated>true</IsTruncated><NextMarker>TH_001/m1</NextMarker><Contents><Key>TH_001/m1</Key><Size>1</Size></Contents></ListBucketResult>"
$versions = "<ListVersionsResult $ns><Name>my-bucket</Name><IsTruncated>false</IsTruncated><Version><Key>a</Key><VersionId>v1</VersionId></Version><Version><Key>b</Key><VersionId>v2</VersionId></Version></ListVersionsResult>"
$deleted = "<DeleteResult $ns><Deleted><Key>a</Key></Deleted><Deleted><Key>b</Key></Deleted></DeleteResult>"
$init = "<InitiateMultipartUploadResult $ns><Bucket>my-bucket</Bucket><Key>k</Key><UploadId>up-1</UploadId></InitiateMultipartUploadResult>"
$complete = "<CompleteMultipartUploadResult $ns><Bucket>my-bucket</Bucket><Key>k</Key><ETag>`"c`"</ETag></CompleteMultipartUploadResult>"
$listDirs = "<ListBucketResult $ns><Name>my-bucket</Name><IsTruncated>false</IsTruncated><CommonPrefixes><Prefix>d1/</Prefix></CommonPrefixes></ListBucketResult>"
$md5 = '"5eb63bbbe01eeed093cb22bb8f5acdc3"'

function New-Route($contains, $status, $body, $headers) {
	$r = [ordered]@{ contains = $contains; status = $status }
	if ($body) { $r.responseBody = $body }
	if ($headers) { $r.responseHeaders = $headers }
	$r
}

# 요청 줄에 처음으로 맞는 경로의 응답을 쓴다.
$std = @(
	(New-Route "?uploads" 200 $init),
	(New-Route "partNumber=" 200 $null @{ ETag = '"p"' }),
	(New-Route "POST /my-bucket/TH" 200 $complete),
	(New-Route "?delete" 200 $deleted),
	(New-Route "DELETE " 204),
	(New-Route "PUT " 200 $null @{ ETag = $md5 }),
	(New-Route "list-type=2" 200 $listV2),
	(New-Route "?versions" 200 $versions),
	(New-Route "GET /my-bucket/?" 200 $listV1Trunc),
	(New-Route "HEAD " 200 $null @{ 'Content-Length' = '11'; 'x-amz-version-id' = 'ver-1' }),
	(New-Route "GET /" 200 "hello world" @{ ETag = $md5 })
)

$error500 = '<Error><Code>InternalError</Code><Message>boom</Message></Error>'
$noSuchKey = '<Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message></Error>'
$cases = [ordered]@{
	'prepare'                  = @{ op = 'prepare'; maxCount = 3 }
	'prepare-check'            = @{ op = 'prepare'; maxCount = 2; check = $true }
	'prepare-start'            = @{ op = 'prepare'; maxCount = 3; start = 1; bucketType = 'One' }
	'prepare-chunked'          = @{ op = 'prepare'; maxCount = 1; useChunkEncoding = $true }
	'prepare-fail'             = @{ op = 'prepare'; maxCount = 3; routes = @((New-Route "PUT " 500 $error500)) }
	'prepare-fail-distributed' = @{ op = 'prepare'; maxCount = 3; distributed = $true; routes = @((New-Route "PUT " 403 '')) }
	'prepare-dir'              = @{ op = 'prepare-dir'; maxCount = 2; bucketType = 'Thread' }
	'head'                     = @{ op = 'head'; maxCount = 2; quitAfter = 3 }
	'read-v2'                  = @{ op = 'read-v2'; maxCount = 2; etagCheck = $true }
	'read-v2-size-mismatch'    = @{ op = 'read-v2'; maxCount = 1; fileSize = 5 }
	'read-v2-md5-mismatch'     = @{ op = 'read-v2'; maxCount = 1; etagCheck = $true; fileContent = 'hello earth' }
	'read-v2-404'              = @{ op = 'read-v2'; maxCount = 1; routes = @((New-Route "GET /" 404 $noSuchKey)) }
	'read-v3'                  = @{ op = 'read-v3'; quitAfter = 3 }
	'read-v3-empty'            = @{ op = 'read-v3'; quitAfter = 5; routes = @((New-Route "list-type=2" 200 $listV2Empty)) }
	'write'                    = @{ op = 'write'; quitAfter = 3; bucketType = 'Time' }
	'write-v2'                 = @{ op = 'write-v2'; maxCount = 2; quitAfter = 3 }
	'delete-bulk'              = @{ op = 'delete'; bulk = $true; maxCount = 0 }
	'delete-each'              = @{ op = 'delete'; bulk = $false; maxCount = 0 }
	'delete-empty'             = @{ op = 'delete'; bulk = $false; maxCount = 0; routes = @((New-Route "list-type=2" 200 $listV2Empty)) }
	'delete-distributed-max'   = @{ op = 'delete'; bulk = $false; maxCount = 1; distributed = $true }
	'delete-v2'                = @{ op = 'delete-v2'; maxCount = 2 }
	'delete-v2-not-204'        = @{ op = 'delete-v2'; maxCount = 1; routes = @((New-Route "DELETE " 200)) }
	'delete-one'               = @{ op = 'delete-one'; key = 'same/key'; maxCount = 2 }
	'delete-version-bulk'      = @{ op = 'delete-version'; bulk = $true; maxCount = 0 }
	'delete-version-each'      = @{ op = 'delete-version'; bulk = $false; maxCount = 0; prefix = 'p/' }
	'mix'                      = @{ op = 'mix'; readRatio = 1; writeRatio = 2; quitAfter = 5; bucketType = 'One' }
	'put-get'                  = @{ op = 'put-get'; quitAfter = 4 }
	'all'                      = @{ op = 'all'; quitAfter = 6 }
	'multi-upload'             = @{ op = 'multi-upload'; maxCount = 1; partSize = 4 }
	'multi-upload-v2'          = @{ op = 'multi-upload-v2'; maxCount = 1; partSize = 6 }
	'multi-upload-part-fail'   = @{ op = 'multi-upload'; maxCount = 1; partSize = 4; routes = @((New-Route "partNumber=2" 500 '')) }
	'upload-tag'               = @{ op = 'upload-tag'; maxCount = 10 }
	'upload-tag-small'         = @{ op = 'upload-tag'; maxCount = 5 }
	'upload-tag-15'            = @{ op = 'upload-tag'; maxCount = 15 }
	'aws-test'                 = @{ op = 'aws-test'; maxCount = 1 }
	'sample-upload'            = @{ op = 'sample-upload'; maxCount = 2; prefix = 's/' }
	'list-object'              = @{ op = 'list-object'; quitAfter = 3 }
	'put-500-no-retry'         = @{ op = 'write'; quitAfter = 1; routes = @((New-Route "PUT " 500 '')) }
	'upload'                   = @{ op = 'upload'; maxCount = 1; partSize = 5242880 }
	'download'                 = @{ op = 'download'; maxCount = 1 }
	'prepare-new'              = @{ op = 'prepare-new'; maxCount = 2 }
	'prepare-random'           = @{ op = 'prepare-random'; maxCount = 2; fileSize = 2000000 }
	'read-new'                 = @{ op = 'read-new'; maxCount = 2 }
	'write-random'             = @{ op = 'write-random'; quitAfter = 2; fileSize = 1500000 }
	'delete-new'               = @{ op = 'delete-new'; maxCount = 2 }
	'delete-directory'         = @{ op = 'delete-directory'; quitAfter = 4; routes = @((New-Route "delimiter=" 200 $listDirs)) }
	'mix-new'                  = @{ op = 'mix-new'; quitAfter = 5 }
	'mix-v2'                   = @{ op = 'mix-v2'; readRatio = 2; writeRatio = 2; deleteRatio = 1; quitAfter = 6 }
}

foreach ($n in $cases.Keys) {
	$c = $cases[$n]
	$c.routes = @($c.routes) + $std | Where-Object { $_ }
	[IO.File]::WriteAllText("$PWD/$d/$n.json", ($c | ConvertTo-Json -Depth 6) + "`n", (New-Object Text.UTF8Encoding $false))
}
dotnet build tools/dotnet-oracle -v q -nologo | Out-Null
foreach ($n in $cases.Keys) {
	$out = dotnet tools/dotnet-oracle/bin/Debug/net10.0/DotnetOracle.dll updown "$d/$n.json"
	[IO.File]::WriteAllText("$PWD/tests/parity/baseline/updown/$n.json", ($out -join "`n") + "`n", (New-Object Text.UTF8Encoding $false))
}
