// UpDownStats 출력 기준 데이터. 사례 JSON의 단계마다 클라이언트 통계를 넣고 Update를 부른 뒤,
// 지정한 Print* 메서드가 남긴 로그 메시지를 그대로 기록한다. 마지막에 ToUpDownResult + SerializeToJson 결과도 남긴다.
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using TestCore.Client;
using TestCore.Data;
using TestCore.Data.Config;

static partial class Program
{
	public sealed class StatsCase
	{
		public long FileSize { get; set; }
		public long PartSize { get; set; }
		public long Total { get; set; }
		public string Config { get; set; }
		public List<StatsStep> Steps { get; set; } = [];
		public StatsResult Result { get; set; }
	}

	public sealed class StatsStep
	{
		public List<Dictionary<string, long[]>> Clients { get; set; } = [];
		public decimal Times { get; set; }
		public List<string> Prints { get; set; } = [];
	}

	public sealed class StatsResult
	{
		public string TestType { get; set; }
		public decimal ExecutionTime { get; set; }
	}

	sealed class FakeClient : ITestClient
	{
		public TestStats Stats { get; } = new();
		public bool Quit { get; set; }
	}

	static void Apply(OperationStats stats, long[] values)
	{
		stats.Success = values[0];
		stats.Error = values[1];
	}

	static string StatsProbe(string casePath)
	{
		var spec = JsonSerializer.Deserialize<StatsCase>(File.ReadAllText(casePath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var repository = log4net.LogManager.CreateRepository("stats-" + Guid.NewGuid());
		var memory = new log4net.Appender.MemoryAppender();
		log4net.Config.BasicConfigurator.Configure(repository, memory);
		var log = log4net.LogManager.GetLogger(repository.Name, "stats");

		var stats = new UpDownStats(spec.FileSize);
		var steps = new List<object>();
		foreach (var step in spec.Steps)
		{
			var clients = new List<ITestClient>();
			foreach (var values in step.Clients)
			{
				var client = new FakeClient();
				if (values.TryGetValue("write", out var w)) { Apply(client.Stats.Write, w); if (w.Length > 2) client.Stats.Write.Part = (int)w[2]; }
				if (values.TryGetValue("read", out var r)) Apply(client.Stats.Read, r);
				if (values.TryGetValue("head", out var h)) Apply(client.Stats.Head, h);
				if (values.TryGetValue("delete", out var d)) Apply(client.Stats.Delete, d);
				if (values.TryGetValue("list", out var l)) Apply(client.Stats.List, l);
				clients.Add(client);
			}
			stats.Update(clients);
			var messages = new List<string>();
			foreach (var print in step.Prints)
			{
				memory.Clear();
				var t = step.Times;
				switch (print)
				{
					case "Prepare": stats.PrintPrepare(log, spec.Total, t); break;
					case "Write": stats.PrintWrite(log, t); break;
					case "Read": stats.PrintRead(log, t); break;
					case "ReadTotal": stats.PrintRead(log, spec.Total, t); break;
					case "ListObject": stats.PrintListObject(log, t); break;
					case "Head": stats.PrintHead(log, t); break;
					case "Delete": stats.PrintDelete(log, t); break;
					case "DeleteTotal": stats.PrintDelete(log, spec.Total, t); break;
					case "MultiUpload": stats.PrintMultiUpload(log, spec.Total, t, spec.PartSize); break;
					case "MultiUploadV2": stats.PrintMultiUploadV2(log, spec.Total, t, spec.PartSize); break;
					case "Download": stats.PrintDownload(log, spec.Total, t); break;
					case "Mix": stats.PrintMix(log, t); break;
					case "All": stats.PrintAll(log, t); break;
					case "AWS": stats.PrintAWS(log, t); break;
					case "PrepareFinal": stats.PrintPrepareFinal(log, spec.Total, t); break;
					case "WriteFinal": stats.PrintWriteFinal(log, t); break;
					case "ReadFinal": stats.PrintReadFinal(log, t); break;
					case "ListObjectFinal": stats.PrintListObjectFinal(log, t); break;
					case "ReadV2Final": stats.PrintReadV2Final(log, spec.Total, t); break;
					case "HeadFinal": stats.PrintHeadFinal(log, t); break;
					case "DeleteFinal": stats.PrintDeleteFinal(log, t); break;
					case "DeleteV2Final": stats.PrintDeleteV2Final(log, spec.Total, t); break;
					case "MixFinal": stats.PrintMixFinal(log, t); break;
					case "AllFinal": stats.PrintAllFinal(log, t); break;
					default: throw new ArgumentException(print);
				}
				messages.AddRange(memory.GetEvents().Select(e => e.RenderedMessage));
			}
			steps.Add(new { messages });
		}

		string resultJson = null;
		if (spec.Result != null)
		{
			var config = new Config();
			config.GetConfig(spec.Config);
			var result = stats.ToUpDownResult(spec.Result.TestType, config.UpDown, config.Main, spec.Result.ExecutionTime);
			result.StartTime = new DateTime(2024, 1, 2, 3, 4, 5, DateTimeKind.Utc);
			result.EndTime = new DateTime(2024, 1, 2, 3, 5, 6, DateTimeKind.Utc);
			resultJson = TestCore.Converter.JsonSerializer.SerializeToJson(result);
		}
		return JsonSerializer.Serialize(new { steps, resultJson }, Json);
	}
}
