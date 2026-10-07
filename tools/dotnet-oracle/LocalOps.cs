// LocalClient 기준 데이터. 임시 디렉터리에서 .NET LocalClient 메서드를 실행하고 통계, WARN 이상 로그
// (첫 줄, 작업 디렉터리는 <WORK>로 바꿈), 메서드 밖으로 나간 예외, 실행 뒤 대상 디렉터리의 파일 목록을 기록한다.
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using TestCore.Client;
using TestCore.Data.Config;

static partial class Program
{
	public sealed class LocalCase
	{
		public List<LocalStep> Steps { get; set; } = [];
		public string FileContent { get; set; } = "hello world";
		public bool DeleteSource { get; set; }
		public List<string> Existing { get; set; } = [];
	}

	public sealed class LocalStep
	{
		public string Op { get; set; }
		public int MaxCount { get; set; }
		public int Start { get; set; }
		public bool Check { get; set; }
		public bool Multipart { get; set; }
		public bool ETagCheck { get; set; }
		public long FileSize { get; set; } = 11;
		public string BucketType { get; set; } = "None";
		public int Thread { get; set; } = 1;
	}

	static string LocalProbe(string casePath)
	{
		var spec = JsonSerializer.Deserialize<LocalCase>(File.ReadAllText(casePath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var repository = log4net.LogManager.GetRepository(typeof(LocalClient).Assembly);
		var memory = new log4net.Appender.MemoryAppender();
		log4net.Config.BasicConfigurator.Configure(repository, memory);

		var work = Path.Combine(Path.GetTempPath(), "oracle-local-" + Guid.NewGuid());
		var target = Path.Combine(work, "target");
		var source = Path.Combine(work, "source.txt");
		Directory.CreateDirectory(work);
		File.WriteAllText(source, spec.FileContent);
		foreach (var existing in spec.Existing)
		{
			var path = Path.Combine(target, existing);
			Directory.CreateDirectory(Path.GetDirectoryName(path));
			File.WriteAllText(path, spec.FileContent);
		}
		if (spec.DeleteSource) File.Delete(source);

		var steps = new List<object>();
		foreach (var step in spec.Steps)
		{
			memory.Clear();
			var config = new UpDownClientConfig("TH", "obj", 1, 1, 0, step.FileSize, Enum.Parse<EnumBucketTypes>(step.BucketType), step.ETagCheck, 1000, 0, false, false);
			var client = new LocalClient(target, step.Thread, source, config, step.Multipart);
			object error = null;
			try
			{
				switch (step.Op)
				{
					case "prepare": client.Prepare(step.MaxCount, step.Check, step.Start); break;
					case "read-v2": client.ReadV2(step.MaxCount, step.Start); break;
					case "read": client.Read(); break;
					case "delete": client.Delete(); break;
					default: throw new ArgumentException(step.Op);
				}
			}
			catch (Exception e) { error = new { type = e.GetType().FullName, message = e.Message.Replace(work, "<WORK>") }; }
			var s = client.Stats;
			var logs = memory.GetEvents()
				.Where(e => e.Level >= log4net.Core.Level.Warn)
				.Select(e => $"{e.Level} {e.RenderedMessage.Replace(work, "<WORK>")}" + (e.ExceptionObject == null ? "" : $" | {e.ExceptionObject.GetType().FullName}"))
				.ToList();
			var files = Directory.Exists(target)
				? Directory.GetFiles(target, "*", SearchOption.AllDirectories).Select(f => Path.GetRelativePath(target, f).Replace('\\', '/') + ":" + new FileInfo(f).Length).OrderBy(f => f, StringComparer.Ordinal).ToList()
				: [];
			steps.Add(new
			{
				quit = client.Quit,
				stats = new { write = new[] { s.Write.Success, s.Write.Error }, read = new[] { s.Read.Success, s.Read.Error }, delete = new[] { s.Delete.Success, s.Delete.Error } },
				logs,
				error,
				files,
			});
		}
		try { Directory.Delete(work, true); } catch { }
		return JsonSerializer.Serialize(new { steps }, Json);
	}
}
