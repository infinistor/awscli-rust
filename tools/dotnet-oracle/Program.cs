// parity 기준 출력 생성기. 사용법:
//   dotnet run -- ini <파일>             IniFile 파싱 결과(섹션·키·원본 값·ToString 값)를 JSON으로 출력
//   dotnet run -- config <파일> [사용자]  Config.GetConfig 결과(Config.ToString JSON)를 출력
//   dotnet run -- checksum <파일>         ChecksumCalculator로 모든 알고리즘의 체크섬을 출력
//   dotnet run -- uri <URL>               System.Uri의 Host·Port·IsDefaultPort·AbsolutePath를 출력
//   dotnet run -- uris <URL 목록 파일>    위 결과를 줄마다 계산해 JSON 배열로 출력
//   dotnet run -- sign <요청 JSON 파일>   Aws4SignerForAuthorizationHeader 서명 결과를 출력
//   dotnet run -- ksan <사례 JSON 파일>   KsanClient 요청을 로컬 서버로 캡처하고 처리 결과(반환값·예외)를 출력
// 실행 시 TestCore.dll과 의존 어셈블리는 TESTCORE_BIN(기본: ../../../TESTCore/bin/TestCore)에서 읽는다.
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.Encodings.Web;
using System.Text.Json;
using TestCore.Data.Config;
using TestCore.Util;

static class Program
{
	static readonly JsonSerializerOptions Json = new() { WriteIndented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping };

	static int Main(string[] args)
	{
		var bin = Environment.GetEnvironmentVariable("TESTCORE_BIN")
			?? Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "..", "TESTCore", "bin", "TestCore"));
		AppDomain.CurrentDomain.AssemblyResolve += (_, e) =>
		{
			var path = Path.Combine(bin, new AssemblyName(e.Name).Name + ".dll");
			return File.Exists(path) ? Assembly.LoadFrom(path) : null;
		};
		Console.OutputEncoding = System.Text.Encoding.UTF8;
		return args.Length >= 2 ? Run(args) : Usage();
	}

	static int Usage()
	{
		Console.Error.WriteLine("usage: ini <file> | config <file> [user] | checksum <file> | uri <url> | sign <request.json>");
		return 2;
	}

	// TestCore 타입 참조를 AssemblyResolve 등록 뒤로 미루기 위해 메서드를 분리한다.
	static int Run(string[] args)
	{
		switch (args[0])
		{
			case "ini": Console.WriteLine(DumpIni(args[1])); return 0;
			case "config":
				var config = new Config();
				var ok = config.GetConfig(args[1], args.Length > 2 ? args[2] : null);
				Console.WriteLine(ok ? config.ToString() : "null");
				return ok ? 0 : 1;
			case "checksum": Console.WriteLine(DumpChecksum(args[1])); return 0;
			case "uri": Console.WriteLine(DumpUri(args[1])); return 0;
			case "uris":
				var lines = File.ReadAllLines(args[1]).Where(l => l.Length > 0).Select(l => JsonSerializer.Deserialize<JsonElement>(DumpUri(l)));
				Console.WriteLine(JsonSerializer.Serialize(lines, Json));
				return 0;
			case "sign": Console.WriteLine(Sign(args[1])); return 0;
			case "ksan": Console.WriteLine(Ksan(args[1])); return 0;
			case "json":
				// TestCore JsonExtensions.ToJsonString 형식 확인용: 파일의 각 줄(문자열)과 고정된 구조를 직렬화한다.
				var strings = File.ReadAllLines(args[1]).Select(l => l.Replace("\\r", "\r").Replace("\\n", "\n").Replace("\\t", "\t").Replace("\\0", "\0").Replace("\\x01", "\u0001").Replace("\\x7f", "\u007f")).ToList();
				Console.Write(TestCore.JsonExtensions.ToJsonString(new
				{
					Strings = strings,
					Empty = Array.Empty<int>(),
					EmptyObject = new { },
					Null = (string)null,
					Numbers = new object[] { 0, -1, int.MaxValue, long.MaxValue, 1.5, 0.1, 1e20, 1e-7, 123456789.125, 1e14, 1e15, 123456789012345.6, 0.0001, 0.00001, -2.5e-10, 1.7976931348623157e308, 5e-324, 100.0, -0.0 },
					Bools = new[] { true, false },
					Nested = new { A = new[] { new { B = 1 } } },
				}));
				return 0;
			default: return Usage();
		}
	}

	static string DumpChecksum(string path)
	{
		var result = new SortedDictionary<string, string>(StringComparer.Ordinal);
		foreach (var algorithm in Enum.GetValues<S3ChecksumAlgorithm>())
			if (algorithm != S3ChecksumAlgorithm.None)
				result[algorithm.ToName()] = ChecksumCalculator.CalculateChecksum(path, algorithm);
		return JsonSerializer.Serialize(result, Json);
	}

	static string DumpUri(string url)
	{
		var uri = new Uri(url);
		return JsonSerializer.Serialize(new { url, host = uri.Host, port = uri.Port, isDefaultPort = uri.IsDefaultPort, absolutePath = uri.AbsolutePath, pathAndQuery = uri.PathAndQuery }, Json);
	}

	public sealed class KsanCase
	{
		public string Op { get; set; }
		public string Bucket { get; set; }
		public string Key { get; set; }
		public string Tag { get; set; }
		public int MaxKeys { get; set; } = 1000;
		public string StorageClass { get; set; }
		public string VersionId { get; set; }
		public int Status { get; set; } = 200;
		public string Body { get; set; } = "";
	}

	// 로컬 포트에 한 번만 응답하는 서버를 띄워 KsanClient가 보낸 요청을 그대로 기록하고,
	// 지정한 응답을 돌려준 뒤 KsanClient의 처리 결과(반환값 또는 예외)를 출력한다.
	static string Ksan(string casePath)
	{
		var spec = JsonSerializer.Deserialize<KsanCase>(File.ReadAllText(casePath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var listener = new System.Net.Sockets.TcpListener(System.Net.IPAddress.Loopback, 0);
		listener.Start();
		var port = ((System.Net.IPEndPoint)listener.LocalEndpoint).Port;
		var server = System.Threading.Tasks.Task.Run(() => Capture(listener, spec));

		object result = null, error = null;
		try
		{
			var client = new TestCore.Client.KsanClient(new UserData($"http://127.0.0.1:{port}", "", "AKIAEXAMPLE", "secretExample"));
			switch (spec.Op)
			{
				case "get-tag-index": result = client.GetBucketTagIndex(spec.Bucket).ToString(); break;
				case "delete-tag-index": client.DeleteBucketTagIndex(spec.Bucket); break;
				case "put-tag-index": client.PutBucketTagIndex(spec.Bucket); break;
				case "list-tag-search": result = client.ListBucketTagSearch(spec.Bucket, spec.Tag, spec.MaxKeys).ToString(); break;
				case "storage-move": client.StorageMove(spec.Bucket, spec.Key, spec.StorageClass, spec.VersionId); break;
				default: throw new ArgumentException(spec.Op);
			}
		}
		catch (Exception e)
		{
			var ksan = e as TestCore.Data.Ksan.KsanException;
			error = new
			{
				type = e.GetType().FullName,
				message = e.Message,
				errorResponse = ksan?.ErrorResponse == null ? null : new { ksan.ErrorResponse.Code, ksan.ErrorResponse.Message, ksan.ErrorResponse.RequestId },
			};
		}
		listener.Stop();
		object request = server.Wait(TimeSpan.FromSeconds(5)) ? server.Result : null;
		return JsonSerializer.Serialize(new { port, request, result, error }, Json);
	}

	static object Capture(System.Net.Sockets.TcpListener listener, KsanCase spec)
	{
		try
		{
			using var socket = listener.AcceptTcpClient();
			using var stream = socket.GetStream();
			var buffer = new List<byte>();
			var one = new byte[1];
			while (!(buffer.Count >= 4 && buffer[^4] == '\r' && buffer[^3] == '\n' && buffer[^2] == '\r' && buffer[^1] == '\n'))
			{
				if (stream.Read(one, 0, 1) == 0) break;
				buffer.Add(one[0]);
			}
			var head = System.Text.Encoding.UTF8.GetString(buffer.ToArray()).Split("\r\n", StringSplitOptions.RemoveEmptyEntries);
			var headers = head.Skip(1).Select(h => { var i = h.IndexOf(':'); return new[] { h[..i], h[(i + 1)..].Trim() }; }).ToList();
			var length = headers.Where(h => h[0].Equals("Content-Length", StringComparison.OrdinalIgnoreCase)).Select(h => int.Parse(h[1])).FirstOrDefault();
			var body = new byte[length];
			for (var read = 0; read < length;) read += stream.Read(body, read, length - read);

			var responseBody = System.Text.Encoding.UTF8.GetBytes(spec.Body ?? "");
			var responseHead = $"HTTP/1.1 {spec.Status} Status\r\nContent-Type: application/xml\r\nContent-Length: {responseBody.Length}\r\nConnection: close\r\n\r\n";
			stream.Write(System.Text.Encoding.ASCII.GetBytes(responseHead));
			stream.Write(responseBody);
			return new { line = head[0], headers, body = System.Text.Encoding.UTF8.GetString(body) };
		}
		catch (Exception e) { return new { error = e.Message }; }
	}

	public sealed class SignRequest
	{
		public string Method { get; set; }
		public string Url { get; set; }
		public string Service { get; set; }
		public string Region { get; set; }
		public string AccessKey { get; set; }
		public string SecretKey { get; set; }
		public string BodyHash { get; set; }
		public string Query { get; set; }
		public Dictionary<string, string> Headers { get; set; } = [];
	}

	// 서명기는 현재 시각을 쓰므로, 결과의 xAmzDate를 Rust 쪽 서명 시각으로 넘겨 비교한다.
	static string Sign(string requestPath)
	{
		var request = JsonSerializer.Deserialize<SignRequest>(File.ReadAllText(requestPath), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
		var headers = new Dictionary<string, string>(request.Headers);
		var signer = new TestCore.Signers.Aws4SignerForAuthorizationHeader
		{
			EndpointUri = new Uri(request.Url),
			HttpMethod = request.Method,
			Service = request.Service,
			Region = request.Region,
		};
		try
		{
			var authorization = signer.ComputeSignature(headers, request.Query, request.BodyHash, request.AccessKey, request.SecretKey, false);
			return JsonSerializer.Serialize(new { headers, authorization }, Json);
		}
		catch (Exception e)
		{
			return JsonSerializer.Serialize(new { headers, error = e.GetType().FullName }, Json);
		}
	}

	static string DumpIni(string path)
	{
		var ini = new IniFile();
		ini.Load(path);
		var sections = new List<object>();
		foreach (var section in ini)
		{
			var values = new List<object>();
			foreach (var kv in section.Value)
				values.Add(new { key = kv.Key, raw = kv.Value.GetString(false, true), text = kv.Value.ToString() });
			sections.Add(new { name = section.Key, values });
		}
		// 대소문자만 다른 이름으로 조회되는지 확인한다.
		var probes = new List<object>();
		foreach (var section in ini)
		{
			var upper = section.Key.ToUpperInvariant();
			var lower = section.Key.ToLowerInvariant();
			probes.Add(new { name = section.Key, upper = ini.ContainsSection(upper), lower = ini.ContainsSection(lower) });
		}
		return JsonSerializer.Serialize(new { sections, probes }, Json);
	}
}
