#!/usr/bin/env bash
# TESTCore(.NET)와 awscli-rest(Rust)를 같은 KSAN·같은 부하로 번갈아 실행해 처리량·평균 지연·최대 메모리를 비교한다.
#
#   ./compare.sh --net-ini testcore.ini --rs-ini awscli-rest.ini [--rounds 3] [--threads 32] [--times 60]
#                [--size 1M] [--files 32] [--net ./testcore/TESTCore] [--rs ./awscli-rest] [--out results]
#
# 각 INI는 접속 정보([Main User])와 버킷 이름([Default] BucketName)만 쓴다. 부하 값은 실행용 사본에서 덮어쓴다.
# 버킷은 도구별로 `{BucketName}-perf-net`·`-perf-rs`로 나누고, 끝나면 비운 뒤 지운다.
# 결과: {out}/summary.md(표·±10% 판정), {out}/runs.csv(원시 값), {out}/logs/*.log. 자격 증명은 남기지 않는다.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
net_ini="" rs_ini="" rounds=3 threads=32 times=60 size=1M files=32
net_exe="$here/testcore/TESTCore" rs_exe="$here/awscli-rest" out="$here/results" tolerance=10
while [ $# -gt 0 ]; do
	case "$1" in
		--net-ini) net_ini=$2; shift 2 ;;
		--rs-ini) rs_ini=$2; shift 2 ;;
		--rounds) rounds=$2; shift 2 ;;
		--threads) threads=$2; shift 2 ;;
		--times) times=$2; shift 2 ;;
		--size) size=$2; shift 2 ;;
		--files) files=$2; shift 2 ;;
		--net) net_exe=$2; shift 2 ;;
		--rs) rs_exe=$2; shift 2 ;;
		--out) out=$2; shift 2 ;;
		*) echo "알 수 없는 인자: $1" >&2; exit 2 ;;
	esac
done
[ -f "$net_ini" ] && [ -f "$rs_ini" ] || { echo "--net-ini, --rs-ini 파일이 필요합니다." >&2; exit 2; }
[ -x "$net_exe" ] && [ -x "$rs_exe" ] || { echo "실행 파일을 찾을 수 없습니다: $net_exe, $rs_exe" >&2; exit 2; }

mkdir -p "$out/logs" "$out/save"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# INI 섹션의 키 값을 바꾸거나, 없으면 섹션 끝(섹션이 없으면 파일 끝)에 넣는다.
ini_set() { # file section key value
	awk -v s="[$2]" -v k="$3" -v v="$4" '
		/^\[/ {
			if (in_s && !done) { print k " = " v; done = 1 }
			in_s = ($0 == s); if (in_s) found = 1
		}
		in_s && $0 ~ "^[ \t]*" k "[ \t]*=" { if (!done) { print k " = " v; done = 1 }; next }
		{ print }
		END {
			if (in_s && !done) { print k " = " v; done = 1 }
			if (!found) { print ""; print s; print k " = " v }
		}
	' "$1" > "$1.tmp" && mv "$1.tmp" "$1"
}

ini_get() { # file section key
	awk -v s="[$2]" -v k="$3" '/^\[/ { in_s = ($0 == s) } in_s && $0 ~ "^[ \t]*" k "[ \t]*=" { sub(/^[^=]*=[ \t]*/, ""); sub(/[ \t\r]*$/, ""); print; exit }' "$1"
}

# 실행용 INI: 원본을 복사하고 부하 값을 덮어쓴다.
make_ini() { # src dst suffix
	sed 's/\r$//' "$1" > "$2"
	local bucket
	bucket=$(ini_get "$1" Default BucketName)
	ini_set "$2" Default BucketName "${bucket}-perf-$3"
	ini_set "$2" Default FileSize "$size"
	ini_set "$2" Default FilePath "$work/data-$3"
	ini_set "$2" UpDown ThreadCount "$threads"
	ini_set "$2" UpDown FileCount "$files"
	ini_set "$2" UpDown Times "$times"
	ini_set "$2" UpDown BucketType 4
	ini_set "$2" UpDown ReadRatio 1
	ini_set "$2" UpDown WriteRatio 1
	ini_set "$2" UpDown DeleteRatio 0
	ini_set "$2" UpDown ETagCheck false
	echo "${bucket}-perf-$3"
}
net_bucket=$(make_ini "$net_ini" "$work/net.ini" net)
rs_bucket=$(make_ini "$rs_ini" "$work/rs.ini" rs)
endpoint=$(ini_get "$rs_ini" "Main User" URL)

# 프로세스를 실행하고 종료 코드·경과 초·최대 RSS(KiB)를 돌려준다. 저장 JSON은 save/{tag}/ 아래.
run_tool() { # tool tag menu
	local tool=$1 tag=$2 menu=$3 exe ini
	if [ "$tool" = net ]; then exe=$net_exe ini=$work/net.ini; else exe=$rs_exe ini=$work/rs.ini; fi
	mkdir -p "$out/save/$tag"
	local start end pid hwm=0 code
	start=$(date +%s.%N)
	"$exe" -c "$ini" "$menu" -s "$out/save/$tag" > "$out/logs/$tag.log" 2>&1 &
	pid=$!
	while kill -0 "$pid" 2>/dev/null; do
		local now
		now=$(awk '/^VmHWM:/ { print $2 }' "/proc/$pid/status" 2>/dev/null || echo 0)
		[ -n "$now" ] && [ "$now" -gt "$hwm" ] && hwm=$now
		sleep 0.5
	done
	wait "$pid" && code=0 || code=$?
	end=$(date +%s.%N)
	echo "$code $(awk -v a="$start" -v b="$end" 'BEGIN { printf "%.3f", b - a }') $hwm"
}

# 저장 JSON의 숫자 필드.
json_num() { # file key
	sed -n "s/^[[:space:]]*\"$2\":[[:space:]]*\(-\{0,1\}[0-9][0-9.]*\).*/\1/p" "$1" | head -1
}

echo "tool,scenario,round,exit,elapsed_s,max_rss_kib,read,read_failed,write,write_failed,ops_per_s,avg_latency_ms" > "$out/runs.csv"
record() { # tool scenario round result
	local tool=$1 scenario=$2 round=$3 code elapsed hwm json r rf w wf ops lat
	read -r code elapsed hwm <<< "$4"
	json=$(ls -t "$out/save/$tool-$scenario-$round"/*.json 2>/dev/null | head -1 || true)
	if [ -n "$json" ]; then
		r=$(json_num "$json" read); rf=$(json_num "$json" readFailed)
		w=$(json_num "$json" write); wf=$(json_num "$json" writeFailed)
	else
		r=0 rf=0 w=0 wf=0
	fi
	# 처리량: 성공 건수 ÷ 설정 시간(두 도구가 같은 정의). 평균 지연: 스레드 수 ÷ 처리량(리틀 법칙).
	ops=$(awk -v r="$r" -v w="$w" -v t="$times" 'BEGIN { printf "%.3f", (r + w) / t }')
	lat=$(awk -v o="$ops" -v n="$threads" 'BEGIN { if (o > 0) printf "%.3f", n / o * 1000; else print "" }')
	echo "$tool,$scenario,$round,$code,$elapsed,$hwm,$r,$rf,$w,$wf,$ops,$lat" >> "$out/runs.csv"
	echo "  $tool $scenario #$round: exit=$code ops/s=$ops avg_ms=$lat rss_kib=$hwm (fail r=$rf w=$wf)"
}

echo "대상: $endpoint, 버킷: $net_bucket / $rs_bucket, 스레드 $threads, ${times}초, 크기 $size, 회차 $rounds"
echo "Prepare(Get 대상 객체 준비)"
for tool in net rs; do
	record "$tool" prepare 0 "$(run_tool "$tool" "$tool-prepare-0" --test-prepare)"
done
for round in $(seq 1 "$rounds"); do
	if [ $((round % 2)) -eq 1 ]; then order="net rs"; else order="rs net"; fi
	for scenario in put get mix; do
		for tool in $order; do
			record "$tool" "$scenario" "$round" "$(run_tool "$tool" "$tool-$scenario-$round" "--test-$scenario")"
		done
	done
done
echo "정리(버킷 비우기·삭제)"
"$net_exe" -c "$work/net.ini" "--bucket-clear=$net_bucket" --flag=true > "$out/logs/net-clear.log" 2>&1 || true
"$rs_exe" -c "$work/rs.ini" "--bucket-clear=$rs_bucket" --flag=true > "$out/logs/rs-clear.log" 2>&1 || true

# 요약: 시나리오별 평균과 Rust/.NET 비율, ±tolerance% 판정.
awk -F, -v tol="$tolerance" -v endpoint="$endpoint" -v threads="$threads" -v times="$times" -v size="$size" -v rounds="$rounds" '
	NR == 1 || $2 == "prepare" { next }
	{
		k = $1 SUBSEP $2; n[k]++; ops[k] += $11; lat[k] += $12; rss[k] = ($6 > rss[k] ? $6 : rss[k])
		fail[k] += $8 + $10; if ($4 != 0) bad[k]++
		if (min[k] == "" || $11 < min[k]) min[k] = $11; if ($11 > max[k]) max[k] = $11
	}
	END {
		print "# TESTCore(.NET) vs awscli-rest(Rust) 성능 비교\n"
		printf "- 대상: %s\n- 부하: 스레드 %s, %s초, 파일 %s, 회차 %s(번갈아 실행)\n", endpoint, threads, times, size, rounds
		printf "- 처리량: 성공 건수 ÷ 설정 시간, 평균 지연: 스레드 ÷ 처리량, 허용 범위 ±%s%%\n\n", tol
		print "| 시나리오 | .NET ops/s (최소~최대) | Rust ops/s (최소~최대) | 비율 | .NET 지연 ms | Rust 지연 ms | 지연 비율 | .NET RSS MiB | Rust RSS MiB | 실패(net/rs) | 판정 |"
		print "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
		split("put get mix", list, " ")
		for (i = 1; i <= 3; i++) {
			s = list[i]; a = "net" SUBSEP s; b = "rs" SUBSEP s
			if (!n[a] || !n[b]) continue
			oa = ops[a] / n[a]; ob = ops[b] / n[b]; la = lat[a] / n[a]; lb = lat[b] / n[b]
			ratio = oa > 0 ? ob / oa : 0; lratio = la > 0 ? lb / la : 0
			ok = (ratio >= 1 - tol / 100 && ratio <= 1 + tol / 100 && lratio >= 1 - tol / 100 && lratio <= 1 + tol / 100 && !fail[a] && !fail[b] && !bad[a] && !bad[b])
			printf "| %s | %.1f (%.1f~%.1f) | %.1f (%.1f~%.1f) | %.3f | %.2f | %.2f | %.3f | %.1f | %.1f | %d/%d | %s |\n", s, oa, min[a], max[a], ob, min[b], max[b], ratio, la, lb, lratio, rss[a] / 1024, rss[b] / 1024, fail[a], fail[b], ok ? "통과" : "범위 밖"
		}
	}' "$out/runs.csv" > "$out/summary.md"
cat "$out/summary.md"
