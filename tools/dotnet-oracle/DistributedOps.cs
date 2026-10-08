using System;
using System.Collections.Generic;
using System.Text.Json;
using TestCore.Data;
using TestCore.Data.Config;
using TestCore.Distributed;

static partial class Program
{
	/// <summary>분산 실행 계약 JSON 예시: 통신용(JsonSerializerDefaults.Web)과 파일용(PascalCase, 들여쓰기).</summary>
	static string Contracts()
	{
		var web = new JsonSerializerOptions(JsonSerializerDefaults.Web);
		var file = new JsonSerializerOptions { WriteIndented = true };
		var request = new TestRequest
		{
			RunId = "0123456789abcdef0123456789abcdef",
			WorkerId = "driver1",
			TestType = "Mix",
			User = new UserData("http://127.0.0.1:9000", "kr-1", "access", "secret"),
			LeaseTimeoutSeconds = 20,
			Workload = new WorkloadSettings
			{
				BucketName = "bucket-a",
				ThreadPrefix = "TH",
				ObjectPrefix = "FILE",
				FileSize = 1024,
				RetryCount = 2,
				IsAdmin = true,
				ThreadCount = 4,
				FileCount = 100,
				Times = 30,
				ReadRatio = 7,
				WriteRatio = 3,
				DeleteRatio = 1,
				BucketType = EnumBucketTypes.Thread,
				DivisionCount = 500,
				ETagCheck = true,
				UseChunkEncoding = true,
				Check = true,
				Start = 5,
				Random = true,
				Bulk = true,
				MaxCount = 9,
			},
		};
		var noUser = new TestRequest { RunId = "fedcba9876543210fedcba9876543210", WorkerId = "w-2", TestType = "Get" };
		var status = new WorkerStatus("driver1", true, null, 15, true);
		var start = new StartRequest(new DateTimeOffset(2026, 10, 8, 1, 2, 3, TimeSpan.Zero).AddTicks(1234567));
		var startWhole = new StartRequest(new DateTimeOffset(2026, 10, 8, 1, 2, 3, TimeSpan.Zero));
		var running = new RunSnapshot
		{
			RunId = request.RunId,
			WorkerId = "driver1",
			TestType = "Mix",
			State = "Running",
			SampleAtUtc = new DateTimeOffset(2026, 10, 8, 1, 2, 5, TimeSpan.Zero).AddTicks(5000000),
			ScheduledAtUtc = start.StartAtUtc,
			StartedAtUtc = start.StartAtUtc.AddTicks(10),
			ElapsedSeconds = 1.5,
			Result = new UpDownResult
			{
				Read = 10, ReadFailed = 1, Head = 2, HeadFailed = 0, Write = 5, WriteFailed = 2, Delete = 3, DeleteFailed = 1,
				List = 4, ListFailed = 1, Total = 30, TotalFailed = 5, Time = 1,
				StartTime = new DateTime(2026, 10, 8, 1, 2, 3, DateTimeKind.Utc).AddTicks(1234567),
				EndTime = default,
				TestType = "Mix", ThreadCount = 4, FileSize = 1024, ReadRatio = 7, WriteRatio = 3, DeleteRatio = 1,
				BucketType = "Thread", BucketName = "bucket-a-driver1", ObjectPrefix = "FILE", ThreadPrefix = "TH/driver1",
			},
		};
		var failed = new RunSnapshot
		{
			RunId = request.RunId,
			WorkerId = "driver1",
			TestType = "Put",
			State = "Failed",
			Error = "Controller heartbeat 만료 \"x\"",
			SampleAtUtc = startWhole.StartAtUtc,
			ElapsedSeconds = 0,
			Result = new UpDownResult { StartTime = default, EndTime = default, TestType = "Unknown" },
		};
		var samples = new Dictionary<string, string>
		{
			["request.web"] = JsonSerializer.Serialize(request, web),
			["request.file"] = JsonSerializer.Serialize(request, file),
			["request-no-user.web"] = JsonSerializer.Serialize(noUser, web),
			["status.web"] = JsonSerializer.Serialize(status, web),
			["start.web"] = JsonSerializer.Serialize(start, web),
			["start-whole.web"] = JsonSerializer.Serialize(startWhole, web),
			["running.web"] = JsonSerializer.Serialize(running, web),
			["running.file"] = JsonSerializer.Serialize(running, file),
			["failed.web"] = JsonSerializer.Serialize(failed, web),
			["failed.file"] = JsonSerializer.Serialize(failed, file),
			["start.o"] = start.StartAtUtc.ToUniversalTime().ToString("O"),
		};
		return JsonSerializer.Serialize(samples, new JsonSerializerOptions { WriteIndented = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping });
	}
}
