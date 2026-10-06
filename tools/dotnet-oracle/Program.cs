// parity 기준 출력 생성기. 사용법:
//   dotnet run -- ini <파일>             IniFile 파싱 결과(섹션·키·원본 값·ToString 값)를 JSON으로 출력
//   dotnet run -- config <파일> [사용자]  Config.GetConfig 결과(Config.ToString JSON)를 출력
// 실행 시 TestCore.dll과 의존 어셈블리는 TESTCORE_BIN(기본: ../../../TESTCore/bin/TestCore)에서 읽는다.
using System;
using System.Collections.Generic;
using System.IO;
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
		Console.Error.WriteLine("usage: ini <file> | config <file> [user]");
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
			default: return Usage();
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
