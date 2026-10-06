// parity 기준 출력 생성기. 사용법:
//   dotnet run -- ini <파일>             IniFile 파싱 결과(섹션·키·원본 값·ToString 값)를 JSON으로 출력
//   dotnet run -- config <파일> [사용자]  Config.GetConfig 결과(Config.ToString JSON)를 출력
//   dotnet run -- checksum <파일>         ChecksumCalculator로 모든 알고리즘의 체크섬을 출력
//   dotnet run -- uri <URL>               System.Uri의 Host·Port·IsDefaultPort·AbsolutePath를 출력
//   dotnet run -- uris <URL 목록 파일>    위 결과를 줄마다 계산해 JSON 배열로 출력
//   dotnet run -- sign <요청 JSON 파일>   Aws4SignerForAuthorizationHeader 서명 결과를 출력
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
		return JsonSerializer.Serialize(new { url, host = uri.Host, port = uri.Port, isDefaultPort = uri.IsDefaultPort, absolutePath = uri.AbsolutePath }, Json);
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
