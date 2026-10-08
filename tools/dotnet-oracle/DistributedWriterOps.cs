using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using TestCore.Data;
using TestCore.Distributed;

static partial class Program
{
	static readonly DateTimeOffset WriterT0 = new(2026, 10, 8, 1, 0, 0, TimeSpan.Zero);
	const string WriterRunId = "0123456789abcdef0123456789abcdef";

	/// <summary>
	/// 분산 Controller의 ResultWriter·ResultConsoleFormatter를 고정 표본으로 돌려 CSV·JSON·콘솔 로그와 경로 규칙을 출력한다.
	/// 사례 정의(표본 수열, 콘솔 입력, 경로 인자)는 Rust 쪽 tests/parity/distributed_controller.rs와 같아야 한다.
	/// </summary>
	static string DistributedWriter(string dir)
	{
		Directory.CreateDirectory(dir);
		var previousCulture = CultureInfo.CurrentCulture;
		CultureInfo.CurrentCulture = CultureInfo.GetCultureInfo("de-DE");
		try
		{
			var output = new Dictionary<string, object>
			{
				["get"] = WriterGet(Path.Combine(dir, "get")),
				["mix"] = WriterMix(Path.Combine(dir, "mix")),
				["console"] = WriterConsole(),
				["paths"] = WriterPaths(Path.Combine(dir, "paths")),
			};
			return JsonSerializer.Serialize(output, Json);
		}
		finally { CultureInfo.CurrentCulture = previousCulture; }
	}

	static DistributedSettings WriterSettings(string dir, int drivers)
	{
		Directory.CreateDirectory(dir);
		var ini = new StringBuilder("[controller]\r\nResultPath = results\r\ndrivers = " + drivers + "\r\n");
		for (var i = 1; i <= drivers; i++) ini.Append($"[driver{i}]\r\nname = w{i}\r\nurl = http://127.0.0.1:{18000 + i}/driver\r\n");
		var path = Path.Combine(dir, "controller.ini");
		File.WriteAllText(path, ini.ToString());
		return DistributedSettings.Load(path, false);
	}

	static object WriterResultSettings(DistributedSettings settings) => new
	{
		Workload = new WorkloadSettings { BucketName = "bucket-a", FileSize = 1024, ThreadCount = 2, FileCount = 3, Times = 5, ReadRatio = 1, WriteRatio = 1 },
		Drivers = settings.Drivers,
		settings.PollIntervalSeconds,
		settings.StartDelaySeconds,
		settings.LeaseTimeoutSeconds,
	};

	/// <summary>log4net 메모리 어펜더로 ResultWriter가 남기는 INFO 메시지를 모은다.</summary>
	static (log4net.Appender.MemoryAppender Appender, Action Restore) CaptureResultLog()
	{
		var logger = (log4net.Repository.Hierarchy.Logger)log4net.LogManager.GetLogger(typeof(ResultWriter)).Logger;
		var appender = new log4net.Appender.MemoryAppender();
		appender.ActivateOptions();
		var configured = logger.Repository.Configured;
		var level = logger.Level;
		logger.Repository.Configured = true;
		logger.Level = log4net.Core.Level.Info;
		logger.AddAppender(appender);
		return (appender, () => { logger.RemoveAppender(appender); logger.Level = level; logger.Repository.Configured = configured; appender.Close(); });
	}

	static Dictionary<string, object> WriterOutput(ResultWriter writer, log4net.Appender.MemoryAppender appender)
	{
		var csv = File.ReadAllBytes(writer.CsvPath);
		var json = File.ReadAllText(writer.JsonPath);
		// Total.SampleAtUtc는 저장 시각(UtcNow)이라 값을 가린다.
		var total = json.IndexOf("\"Total\": {", StringComparison.Ordinal);
		json = json[..total] + new Regex("\"SampleAtUtc\": \"[^\"]*\"").Replace(json[total..], "\"SampleAtUtc\": \"<NOW>\"", 1);
		return new Dictionary<string, object>
		{
			["csv"] = new UTF8Encoding(false).GetString(csv),
			["csvBom"] = csv.Take(3).SequenceEqual(new byte[] { 239, 187, 191 }),
			["json"] = json,
			["messages"] = appender.GetEvents().Select(e => e.RenderedMessage).ToArray(),
		};
	}

	static RunSnapshot WriterSnapshot(string worker, string type, string state, DateTimeOffset sample, DateTimeOffset? started, DateTimeOffset? completed,
		UpDownResult result, string error = null, DateTimeOffset? issuingStopped = null)
	{
		result.StartTime = (started ?? WriterT0).UtcDateTime;
		result.EndTime = default;
		result.TestType = type;
		return new RunSnapshot
		{
			RunId = WriterRunId,
			WorkerId = worker,
			TestType = type,
			State = state,
			Error = error,
			SampleAtUtc = sample,
			StartedAtUtc = started,
			IssuingStoppedAtUtc = issuingStopped,
			CompletedAtUtc = completed,
			ElapsedSeconds = started.HasValue ? Math.Max(0, ((completed ?? sample) - started.Value).TotalSeconds) : 0,
			Result = result,
		};
	}

	static Dictionary<string, object> WriterGet(string dir)
	{
		var settings = WriterSettings(dir, 2);
		var (appender, restore) = CaptureResultLog();
		try
		{
			using var writer = new ResultWriter(settings, null, WriterRunId, "Get");
			var at = WriterT0;
			RunSnapshot S(string worker, long count, DateTimeOffset time, bool done = false) => WriterSnapshot(worker, "Get", done ? "Completed" : "Running",
				time, at, done ? time : null, new UpDownResult { Read = count, FileSize = 1024 });
			writer.Sample([new("w1", S("w1", 0, at)), new("w2", S("w2", 0, at))], at);
			writer.Sample([new("w1", S("w1", 3, at.AddSeconds(2))), new("w2", S("w2", 5, at.AddSeconds(2)))], at.AddSeconds(2));
			writer.Sample([new("w1", null, "bad,\"quoted\"\nerror"), new("w2", S("w2", 5, at.AddSeconds(4)))], at.AddSeconds(4));
			writer.Sample([new("w1", S("w1", 3, at.AddSeconds(6), true)), new("w2", S("w2", 5, at.AddSeconds(6), true))], at.AddSeconds(6));
			writer.SaveFinal("Completed", null, WriterResultSettings(settings));
			writer.Dispose();
			return WriterOutput(writer, appender);
		}
		finally { restore(); }
	}

	static Dictionary<string, object> WriterMix(string dir)
	{
		var settings = WriterSettings(dir, 3);
		var (appender, restore) = CaptureResultLog();
		try
		{
			using var writer = new ResultWriter(settings, null, WriterRunId, "Mix");
			var t = WriterT0;
			UpDownResult R(long read, long write, long head = 0, long delete = 0, long list = 0, long readFailed = 0, long writeFailed = 0, long listFailed = 0, int threads = 2) =>
				new() { Read = read, Write = write, Head = head, Delete = delete, List = list, ReadFailed = readFailed, WriteFailed = writeFailed, ListFailed = listFailed, ThreadCount = threads, FileSize = 1048576 };
			RunSnapshot M(string worker, string state, DateTimeOffset sample, DateTimeOffset? started, UpDownResult result, DateTimeOffset? completed = null, string error = null, DateTimeOffset? stopped = null) =>
				WriterSnapshot(worker, "Mix", state, sample, started, completed, result, error, stopped);
			var s1 = t.AddMilliseconds(100);
			writer.Sample([new("w1", M("w1", "Preparing", t, null, R(0, 0))), new("w2", M("w2", "Ready", t, null, R(0, 0))), new("w3", M("w3", "Preparing", t, null, R(0, 0)))], t);
			writer.Sample([new("w1", M("w1", "Running", t.AddSeconds(2), t.AddSeconds(1), R(10, 20, 1, 2, 3, 0, 1, 1))),
				new("w2", M("w2", "Running", t.AddSeconds(2), s1.AddSeconds(1), R(7, 9, 0, 0, 0))),
				WorkerSampleFailed("w3", "Worker 조회 실패: HttpRequestException")], t.AddSeconds(2));
			writer.Sample([new("w1", M("w1", "Running", t.AddSeconds(4), t.AddSeconds(1), R(30, 41, 1, 5, 3, 0, 1, 1))),
				new("w2", M("w2", "Running", t.AddSeconds(4), s1.AddSeconds(1), R(5, 19, 0, 0, 0))),
				new("w3", M("w3", "Running", t.AddSeconds(4), t.AddSeconds(3), R(1, 1, threads: 4)))], t.AddSeconds(4));
			writer.Sample([new("w1", M("w1", "Cancelled", t.AddSeconds(6), t.AddSeconds(1), R(31, 42, 1, 5, 3, 0, 1, 1), t.AddSeconds(6), "stopped", t.AddSeconds(5))),
				new("w2", M("w2", "Failed", t.AddSeconds(6), s1.AddSeconds(1), R(5, 19, 0, 0, 0), t.AddSeconds(5.5), "x,y")),
				new("w3", M("w3", "Completed", t.AddSeconds(6), t.AddSeconds(3), R(2, 2, threads: 4), t.AddSeconds(6), null, t.AddSeconds(5.5)))], t.AddSeconds(6));
			writer.Sample([new("w1", M("w1", "Completed", t.AddSeconds(8), t.AddSeconds(1), R(31, 42, 1, 5, 3, 0, 1, 1), t.AddSeconds(8))),
				new("w2", M("w2", "Completed", t.AddSeconds(8), s1.AddSeconds(1), R(5, 19, 0, 0, 0), t.AddSeconds(8))),
				WorkerSampleFailed("w3", "Worker 조회 실패: TaskCanceledException")], t.AddSeconds(8));
			writer.SaveFinal("Cancelled", "사용자 중단", WriterResultSettings(settings));
			writer.Dispose();
			return WriterOutput(writer, appender);
		}
		finally { restore(); }
	}

	static WorkerSample WorkerSampleFailed(string worker, string error) => new(worker, null, error);

	static Dictionary<string, object> WriterConsole()
	{
		string F(RunSnapshot s, int reported, int expected, bool final, double?[] rates = null, string state = null, string error = null, long?[] increments = null) =>
			ResultConsoleFormatter.Format(s, reported, expected, final, rates, state, error, increments);
		RunSnapshot C(string type, string state, double elapsed, UpDownResult result) => new() { TestType = type, State = state, ElapsedSeconds = elapsed, Result = result };
		var cases = new Dictionary<string, object>();
		cases["mix-progress"] = F(C("Mix", "Running", 4, new() { Read = 20, Write = 50 }), 2, 2, false, increments: [0, 12, null, null, null]);
		cases["mix-progress-rates"] = F(C("Mix", "Running", 4, new() { Read = 20, Write = 50, FileSize = 2048 }), 2, 2, false,
			[5.5, 12.25, null, null, null], increments: [11, 49, null, null, null]);
		cases["mix-progress-missing-rate"] = F(C("Mix", "Running", 4, new() { Read = 20, Write = 50, FileSize = 2048 }), 1, 2, false,
			[5.5, null, null, null, null], error: "일부 Worker 통계 누락", increments: [1, null, null, null, null]);
		cases["mix-cancelled"] = F(C("Mix", "Running", 2.5, new() { Read = 10, Write = 20, WriteFailed = 2, FileSize = 1024 * 1024 }), 2, 3, true, state: "Cancelled", error: "사용자 중단");
		cases["get-progress"] = F(C("Get", "Running", 1, new() { Read = 8, ReadFailed = 1, FileSize = 1024 }), 1, 2, false, [4.0, null, null, null, null], increments: [8, null, null, null, null]);
		cases["get-progress-no-input"] = F(C("Get", "Running", 1, new() { Read = 8 }), 2, 2, false);
		cases["get-final"] = F(C("Get", "Completed", 6, new() { Read = 8, FileSize = 1024 }), 2, 2, true, state: "Completed");
		cases["delete-progress"] = F(C("Delete", "Running", 3, new() { Delete = 100, DeleteFailed = 3 }), 2, 2, false, [null, null, null, 33.333333333333336, null], increments: [null, null, null, 100, null]);
		cases["put-final-rounding"] = F(C("Put", "Completed", 3, new() { Write = 1, WriteFailed = 2, FileSize = 5 }), 2, 2, true, state: "Completed", error: "  ");
		foreach (var type in new[] { "Put", "Prepare", "Delete", "Get", "Mix" })
			cases["empty-failed-" + type] = F(new RunSnapshot { TestType = type, State = "Failed" }, 0, 2, true);
		cases["prepare-progress"] = F(C("Prepare", "Preparing", 0, new()), 0, 2, false);
		return cases;
	}

	static Dictionary<string, object> WriterPaths(string dir)
	{
		var cases = new Dictionary<string, object>();
		var index = 0;
		void Case(string name, string save, Action<string> prepare = null)
		{
			var caseDir = Path.Combine(dir, "c" + index++);
			var settings = WriterSettings(caseDir, 2);
			prepare?.Invoke(caseDir);
			string Rel(string p) => Path.GetRelativePath(caseDir, p).Replace('\\', '/');
			try
			{
				using var writer = new ResultWriter(settings, save?.Replace("{ABS}", caseDir.Replace('\\', '/')), WriterRunId, "Get");
				cases[name] = new Dictionary<string, object> { ["json"] = Rel(writer.JsonPath), ["csv"] = Rel(writer.CsvPath), ["files"] = Files(caseDir) };
			}
			catch (Exception e)
			{
				cases[name] = new Dictionary<string, object> { ["error"] = e.GetType().Name, ["message"] = e is IOException || e is ArgumentException ? e.Message.Replace(caseDir, "<DIR>") : "", ["files"] = Files(caseDir) };
			}
		}
		string[] Files(string caseDir) => Directory.GetFileSystemEntries(caseDir, "*", SearchOption.AllDirectories)
			.Select(p => Path.GetRelativePath(caseDir, p).Replace('\\', '/')).Where(p => p != "controller.ini").OrderBy(p => p, StringComparer.Ordinal).ToArray();
		var run = WriterRunId;
		Case("default", null);
		Case("empty", "");
		Case("blank", "  ");
		Case("dir", "out");
		Case("dir-dotted", "dir.d/name");
		Case("json", "out/x.json");
		Case("json-upper", "out/x.JSON");
		Case("csv-extension", "out/y.csv");
		Case("trailing-dot", "out/name.");
		Case("hidden", "out/.json");
		Case("double-extension", "out/a.b.json");
		Case("absolute", "{ABS}/abs/z.json");
		Case("json-exists", "out/x.json", d => { Directory.CreateDirectory(Path.Combine(d, "out")); File.WriteAllText(Path.Combine(d, "out", "x.json"), "old"); });
		Case("csv-exists", "out/x.json", d => { Directory.CreateDirectory(Path.Combine(d, "out")); File.WriteAllText(Path.Combine(d, "out", "x.csv"), "old"); });
		Case("default-json-exists", null, d => { Directory.CreateDirectory(Path.Combine(d, "results")); File.WriteAllText(Path.Combine(d, "results", run + ".json"), "old"); });
		// CSV를 만들 수 없으면 이미 만든 JSON을 지운다(TESTCore ec427f2). File.Exists는 디렉터리에 false라 충돌 검사를 통과한다.
		Case("csv-is-directory", "out/x.json", d => Directory.CreateDirectory(Path.Combine(d, "out", "x.csv")));
		return cases;
	}
}
