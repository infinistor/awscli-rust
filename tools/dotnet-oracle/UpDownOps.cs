// UpDownClient 기준 데이터. 로컬 캡처 서버에 대해 .NET UpDownClient의 테스트 메서드를 실행하고
// 요청 순서, 통계, ERROR 로그, 메서드 밖으로 나간 예외를 기록한다.
// 끝없이 도는 메서드(Write, Head, Mix ...)는 서버가 `QuitAfter`번째 요청을 받으면 응답하기 전에 Quit을 켠다.
using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using TestCore.Client;
using TestCore.Data.Config;

static partial class Program
{
	public sealed class UpDownCase
	{
		public string Op { get; set; }
		public int MaxCount { get; set; }
		public int Start { get; set; }
		public bool Check { get; set; }
		public bool Bulk { get; set; }
		public long PartSize { get; set; }
		public string Prefix { get; set; }
		public string Key { get; set; }
		public int QuitAfter { get; set; }
		public string BucketType { get; set; } = "None";
		public long FileSize { get; set; } = 11;
		public string FileContent { get; set; } = "hello world";
		public int ReadRatio { get; set; } = 1;
		public int WriteRatio { get; set; } = 1;
		public int DeleteRatio { get; set; }
		public bool ETagCheck { get; set; }
		public bool UseChunkEncoding { get; set; }
		public bool Distributed { get; set; }
		public int Retry { get; set; }
		public int Status { get; set; } = 200;
		public string ResponseBody { get; set; } = "";
		public Dictionary<string, string> ResponseHeaders { get; set; } = [];
		public List<S3Route> Routes { get; set; } = [];
	}

	static string UpDownProbe(string casePath)
	{
		var spec = JsonSerializer.Deserialize<UpDownCase>(File.ReadAllText(casePath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var repository = log4net.LogManager.GetRepository(typeof(UpDownClient).Assembly);
		var memory = new log4net.Appender.MemoryAppender();
		log4net.Config.BasicConfigurator.Configure(repository, memory);

		var work = Path.Combine(Path.GetTempPath(), "oracle-updown-" + Guid.NewGuid());
		Directory.CreateDirectory(work);
		var filePath = Path.Combine(work, "body.txt");
		File.WriteAllText(filePath, spec.FileContent);

		var listener = new TcpListener(IPAddress.Loopback, 0);
		listener.Start();
		var port = ((IPEndPoint)listener.LocalEndpoint).Port;
		var requests = new ConcurrentQueue<object>();
		var count = 0;
		UpDownClient client = null;
		var cts = new CancellationTokenSource();
		var server = System.Threading.Tasks.Task.Run(async () =>
		{
			while (!cts.IsCancellationRequested)
			{
				TcpClient socket;
				try { socket = await listener.AcceptTcpClientAsync(cts.Token); } catch { break; }
				_ = System.Threading.Tasks.Task.Run(() => ServeUpDown(socket, spec, requests, () =>
				{
					if (Interlocked.Increment(ref count) == spec.QuitAfter && client != null) client.Quit = true;
				}));
			}
		});

		var bucketType = Enum.Parse<EnumBucketTypes>(spec.BucketType);
		var config = new UpDownClientConfig("TH", "obj", spec.ReadRatio, spec.WriteRatio, spec.DeleteRatio, spec.FileSize, bucketType, spec.ETagCheck, 1000, spec.Retry, false, spec.UseChunkEncoding) { Distributed = spec.Distributed };
		client = new UpDownClient("my-bucket", 1, filePath, config, new UserData($"http://127.0.0.1:{port}", "", "AKIAEXAMPLE", "secretExample"));

		object error = null;
		try
		{
			switch (spec.Op)
			{
				case "prepare": client.Prepare(spec.MaxCount, spec.Check, spec.Start); break;
				case "prepare-dir": client.PrepareDir(spec.MaxCount, spec.Check, spec.Start); break;
				case "prepare-new": client.PrepareNew(spec.MaxCount, spec.Check, spec.Start); break;
				case "prepare-random": client.PrepareRandom(spec.MaxCount, spec.Start); break;
				case "read-new": client.ReadNew(spec.MaxCount, spec.Start); break;
				case "write-random": client.WriteRandom(spec.Start); break;
				case "delete-new": client.DeleteNew(spec.MaxCount, spec.Start); break;
				case "delete-directory": client.DeleteDirectory(); break;
				case "mix-new": client.MixNew(); break;
				case "head": client.Head(spec.MaxCount, spec.Start); break;
				case "read-v2": client.ReadV2(spec.MaxCount, spec.Start); break;
				case "read-v3": client.ReadV3(); break;
				case "write": client.Write(spec.Start); break;
				case "write-v2": client.WriteV2(spec.MaxCount); break;
				case "delete": client.Delete(spec.Bulk, spec.MaxCount); break;
				case "delete-v2": client.DeleteV2(spec.MaxCount, spec.Start); break;
				case "delete-one": client.DeleteOne(spec.Key, spec.MaxCount); break;
				case "delete-version": client.DeleteVersion(spec.Bulk, spec.MaxCount, spec.Prefix); break;
				case "mix": client.Mix(); break;
				case "mix-v2": client.MixV2(); break;
				case "put-get": client.PutGet(); break;
				case "all": client.All(); break;
				case "multi-upload": client.MultiUpload(spec.MaxCount, spec.PartSize); break;
				case "multi-upload-v2": client.MultiUploadV2(spec.MaxCount, spec.PartSize); break;
				case "upload": client.Upload(spec.MaxCount, spec.PartSize); break;
				case "download": client.Download(spec.MaxCount); break;
				case "upload-tag": client.UploadTag(spec.MaxCount); break;
				case "aws-test": client.AWSTest(spec.MaxCount); break;
				case "sample-upload": client.SampleUpload(spec.Prefix, spec.MaxCount); break;
				case "list-object": client.ListObject(); break;
				default: throw new ArgumentException(spec.Op);
			}
		}
		catch (Exception e)
		{
			error = new { type = e.GetType().FullName, message = e.Message };
		}
		Thread.Sleep(200);
		cts.Cancel();
		listener.Stop();
		try { Directory.Delete(work, true); } catch { }

		var s = client.Stats;
		var stats = new
		{
			write = new[] { s.Write.Success, s.Write.Error, s.Write.Part },
			read = new[] { s.Read.Success, s.Read.Error },
			head = new[] { s.Head.Success, s.Head.Error },
			delete = new[] { s.Delete.Success, s.Delete.Error },
			list = new[] { s.List.Success, s.List.Error },
			loopEnd = s.LoopEndCount,
		};
		var logs = memory.GetEvents()
			.Where(e => e.Level >= log4net.Core.Level.Error)
			.Select(e => e.RenderedMessage.Split('\n')[0].TrimEnd('\r'))
			.ToList();
		return JsonSerializer.Serialize(new { port, quit = client.Quit, requests = requests.ToArray(), stats, logs, error }, Json);
	}

	static void ServeUpDown(TcpClient socket, UpDownCase spec, ConcurrentQueue<object> requests, Action onRequest)
	{
		using var _ = socket;
		using var stream = socket.GetStream();
		var one = new byte[1];
		string ReadLine()
		{
			var line = new List<byte>();
			while (!(line.Count >= 2 && line[^2] == '\r' && line[^1] == '\n'))
			{
				if (stream.Read(one, 0, 1) == 0) return null;
				line.Add(one[0]);
			}
			return Encoding.UTF8.GetString(line.ToArray());
		}
		while (true)
		{
			var head = new List<string>();
			while (true)
			{
				var line = ReadLine();
				if (line == null) return;
				if (line == "\r\n") break;
				head.Add(line.TrimEnd('\r', '\n'));
			}
			var headers = head.Skip(1).Select(h => { var i = h.IndexOf(':'); return new[] { h[..i], h[(i + 1)..].Trim() }; }).ToList();
			string Header(string name) => headers.Where(h => h[0].Equals(name, StringComparison.OrdinalIgnoreCase)).Select(h => h[1]).FirstOrDefault();
			if (Header("Expect") == "100-continue") stream.Write(Encoding.ASCII.GetBytes("HTTP/1.1 100 Continue\r\n\r\n"));
			if (Header("Transfer-Encoding") == "chunked")
			{
				while (true)
				{
					var size = Convert.ToInt32(ReadLine().Trim().Split(';')[0], 16);
					if (size == 0) { while (ReadLine() != "\r\n") { } break; }
					var chunk = new byte[size + 2];
					for (var read = 0; read < chunk.Length;) read += stream.Read(chunk, read, chunk.Length - read);
				}
			}
			else
			{
				var length = long.Parse(Header("Content-Length") ?? "0");
				var body = new byte[length];
				for (var read = 0; read < length;) read += stream.Read(body, read, (int)(length - read));
			}
			var line0 = head[0];
			requests.Enqueue(line0);
			onRequest();

			var route = spec.Routes.FirstOrDefault(r => line0.Contains(r.Contains));
			var status = route?.Status ?? spec.Status;
			var responseBody = Encoding.UTF8.GetBytes((route?.ResponseBody ?? spec.ResponseBody) ?? "");
			var extra = string.Concat((route?.ResponseHeaders ?? spec.ResponseHeaders).Select(h => $"{h.Key}: {h.Value}\r\n"));
			var isHead = line0.StartsWith("HEAD ");
			var hasLength = (route?.ResponseHeaders ?? spec.ResponseHeaders).Keys.Any(k => k.Equals("Content-Length", StringComparison.OrdinalIgnoreCase));
			var lengthHeader = hasLength ? "" : $"Content-Length: {responseBody.Length}\r\n";
			stream.Write(Encoding.ASCII.GetBytes($"HTTP/1.1 {status} Status\r\n{extra}{lengthHeader}\r\n"));
			if (!isHead) stream.Write(responseBody);
		}
	}
}
