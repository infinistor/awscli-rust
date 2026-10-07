// MultiSystemClient 기준 데이터. 게이트웨이·구 시스템·신 시스템마다 캡처 서버를 하나씩 띄우고
// .NET MultiSystemClient 메서드를 실행해 (시스템, 요청 줄) 순서, 카운터, ERROR 로그 첫 줄, 예외를 기록한다.
using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text.Json;
using System.Threading;
using TestCore.Client;
using TestCore.Data.Config;

static partial class Program
{
	public sealed class MultiSystemCase
	{
		public string Op { get; set; }
		public int FileCount { get; set; } = 2;
		public long FileSize { get; set; } = 11;
		public long PartSize { get; set; } = 4;
		public string BucketType { get; set; } = "None";
		public string FileContent { get; set; } = "hello world";
		public Dictionary<string, List<S3Route>> Systems { get; set; } = [];
	}

	static string MultiSystemProbe(string casePath)
	{
		var spec = JsonSerializer.Deserialize<MultiSystemCase>(File.ReadAllText(casePath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var repository = log4net.LogManager.GetRepository(typeof(MultiSystemClient).Assembly);
		var memory = new log4net.Appender.MemoryAppender();
		log4net.Config.BasicConfigurator.Configure(repository, memory);

		var work = Path.Combine(Path.GetTempPath(), "oracle-multi-" + Guid.NewGuid());
		Directory.CreateDirectory(work);
		var filePath = Path.Combine(work, "body.txt");
		File.WriteAllText(filePath, spec.FileContent);

		var requests = new ConcurrentQueue<string>();
		var cts = new CancellationTokenSource();
		var clients = new Dictionary<string, TestCore.Client.S3Client>();
		var listeners = new List<TcpListener>();
		foreach (var name in new[] { "gateway", "old", "new" })
		{
			var listener = new TcpListener(IPAddress.Loopback, 0);
			listener.Start();
			listeners.Add(listener);
			var port = ((IPEndPoint)listener.LocalEndpoint).Port;
			var routes = spec.Systems.TryGetValue(name, out var r) ? r : [];
			var system = new UpDownCase { Routes = routes };
			var log = new ConcurrentQueue<object>();
			System.Threading.Tasks.Task.Run(async () =>
			{
				while (!cts.IsCancellationRequested)
				{
					TcpClient socket;
					try { socket = await listener.AcceptTcpClientAsync(cts.Token); } catch { break; }
					// ServeUpDown이 요청 줄을 log에 넣은 직후(응답 전) 시스템 이름을 붙여 옮긴다.
					_ = System.Threading.Tasks.Task.Run(() => ServeUpDown(socket, system, log, () =>
					{
						while (log.TryDequeue(out var line)) requests.Enqueue($"{name} {line}");
					}));
				}
			});
			clients[name] = new TestCore.Client.S3Client(new UserData($"http://127.0.0.1:{port}", "", "AKIAEXAMPLE", "secretExample"), retryCount: 0);
		}

		var config = new MultiSystemClientConfig("TH", "obj", spec.FileCount, spec.FileSize, spec.PartSize, Enum.Parse<EnumBucketTypes>(spec.BucketType));
		var client = new MultiSystemClient(config, "my-bucket", 1, filePath, clients["gateway"], clients["old"], clients["new"]);
		object error = null;
		try
		{
			switch (spec.Op)
			{
				case "prepare": client.Prepare(); break;
				case "prepare-multipart": client.PrepareMultipart(); break;
				case "get": client.Get(); break;
				case "put-get": client.PutGet(); break;
				case "put-get-multipart": client.PutGetMultipart(); break;
				case "mix": client.Mix(); break;
				case "mix-multipart": client.MixMultipart(); break;
				default: throw new ArgumentException(spec.Op);
			}
		}
		catch (Exception e) { error = new { type = e.GetType().FullName, message = e.Message }; }
		Thread.Sleep(300);
		cts.Cancel();
		foreach (var listener in listeners) listener.Stop();
		try { Directory.Delete(work, true); } catch { }

		var counters = new[] { client.WriteCount, client.WriteErrorCount, client.ReadCount, client.ReadErrorCount, client.DeleteCount, client.DeleteErrorCount, client.PartCount, client.ObjectCount };
		var logs = memory.GetEvents()
			.Where(e => e.Level >= log4net.Core.Level.Error)
			.Select(e => e.RenderedMessage.Split('\n')[0].TrimEnd('\r') + (e.ExceptionObject == null ? "" : $" | {e.ExceptionObject.GetType().FullName}"))
			.ToList();
		return JsonSerializer.Serialize(new { quit = client.Quit, requests = requests.ToArray(), counters, logs, error }, Json);
	}
}
