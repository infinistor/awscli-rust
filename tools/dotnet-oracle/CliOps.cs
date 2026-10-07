// CLI 옵션 파서(Mono.Options) 기준 데이터.
//   cli-options             OptionSet에 등록된 옵션(선언 순서): 프로토타입, 이름들, 설명, 값 종류, 값 형식
//   cli-help                WriteOptionDescriptions 원문
//   cli-parse <cases.json>  인자 배열 목록마다 CliOptionParser.Parse 결과(옵션 값, Extra, 예외)
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Mono.Options;
using TestCore.Cli;

static partial class Program
{
	static string CliOptions()
	{
		var parser = new CliOptionParser();
		parser.Parse([]);
		var list = parser.OptionSet.Select(o =>
		{
			var type = o.GetType();
			var valueType = type.IsGenericType ? type.GetGenericArguments()[0].Name : "String";
			return new
			{
				prototype = o.Prototype,
				names = o.GetNames(),
				description = o.Description,
				valueKind = o.OptionValueType.ToString(),
				valueType,
			};
		}).ToList();
		return JsonSerializer.Serialize(list, Json);
	}

	static string CliHelp()
	{
		var parser = new CliOptionParser();
		parser.Parse([]);
		var writer = new StringWriter();
		parser.OptionSet.WriteOptionDescriptions(writer);
		return writer.ToString();
	}

	static object DumpOptions(CommandOptions o) => new Dictionary<string, object>
	{
		["Help"] = o.Help, ["Worker"] = o.Worker, ["Controller"] = o.Controller, ["Version"] = o.Version,
		["ConfigPath"] = o.ConfigPath, ["BucketName"] = o.BucketName, ["Source"] = o.Source, ["Target"] = o.Target,
		["Key"] = o.Key, ["SourceKey"] = o.SourceKey, ["FilePath"] = o.FilePath, ["Path"] = o.Path, ["Tags"] = o.Tags,
		["StrACL"] = o.StrACL, ["Versioning"] = o.Versioning, ["VersionId"] = o.VersionId, ["StorageClass"] = o.StorageClass,
		["Ownership"] = o.Ownership?.Value, ["LockMode"] = o.LockMode, ["Body"] = o.Body, ["Url"] = o.Url,
		["AccessKey"] = o.AccessKey, ["SecretKey"] = o.SecretKey, ["EncryptionKey"] = o.EncryptionKey, ["UserName"] = o.UserName,
		["BucketType"] = (int)o.BucketType, ["Another"] = o.Another, ["Print"] = o.Print, ["ALL"] = o.ALL, ["Check"] = o.Check,
		["Bypass"] = o.Bypass, ["Flag"] = o.Flag, ["Random"] = o.Random, ["Multipart"] = o.Multipart, ["Debug"] = o.Debug,
		["Checksum"] = o.Checksum, ["ChecksumType"] = o.ChecksumType.ToString(), ["Md5sum"] = o.Md5sum, ["Id"] = o.Id,
		["Bulk"] = o.Bulk, ["ThreadPrefix"] = o.ThreadPrefix, ["Admin"] = o.Admin, ["UseChunkEncoding"] = o.UseChunkEncoding,
		["Prefix"] = o.Prefix, ["Suffix"] = o.Suffix, ["Delimiter"] = o.Delimiter, ["Darker"] = o.Darker,
		["ContinuationToken"] = o.ContinuationToken, ["MaxKeys"] = o.MaxKeys, ["Days"] = o.Days, ["Years"] = o.Years,
		["Date"] = o.Date, ["UploadId"] = o.UploadId, ["PartNumber"] = o.PartNumber, ["PartSize"] = o.PartSize,
		["StartByte"] = o.StartByte, ["EndByte"] = o.EndByte, ["FileSize"] = o.FileSize, ["RangeList"] = o.RangeList,
		["StartCount"] = o.StartCount, ["Thread"] = o.Thread, ["Count"] = o.Count, ["Times"] = o.Times, ["Read"] = o.Read,
		["Write"] = o.Write, ["Delete"] = o.Delete, ["Save"] = o.Save, ["ServiceType"] = o.ServiceType, ["Address"] = o.Address,
		["Port"] = o.Port, ["TargetPath"] = o.TargetPath, ["Menu"] = o.Menu.ToString(),
	};

	// 기본값과 다른 항목만 남긴다.
	static Dictionary<string, object> Changed(object dump)
	{
		var defaults = (Dictionary<string, object>)DumpOptions(new CommandOptions());
		return ((Dictionary<string, object>)dump)
			.Where(kv => JsonSerializer.Serialize(kv.Value) != JsonSerializer.Serialize(defaults[kv.Key]))
			.ToDictionary(kv => kv.Key, kv => kv.Value);
	}

	static string CliParse(string casesPath)
	{
		var cases = JsonSerializer.Deserialize<List<string[]>>(File.ReadAllText(casesPath));
		var results = new List<object>();
		foreach (var args in cases)
		{
			var parser = new CliOptionParser();
			try
			{
				var result = parser.Parse(args);
				results.Add(new { args, options = Changed(DumpOptions(result.CommandOptions)), extra = result.Extra, error = (object)null });
			}
			catch (OptionException e)
			{
				results.Add(new { args, options = (object)null, extra = (object)null, error = (object)new { type = e.GetType().FullName, message = e.Message, optionName = e.OptionName } });
			}
			catch (Exception e)
			{
				results.Add(new { args, options = (object)null, extra = (object)null, error = (object)new { type = e.GetType().FullName, message = e.Message, optionName = (string)null } });
			}
		}
		return JsonSerializer.Serialize(results, Json);
	}
}

static partial class CliUsage
{
	// Usage의 공개 상수·정적 필드(선언 순서)를 이름과 값으로 내보낸다.
	public static string Dump()
	{
		var fields = typeof(TestCore.Data.Usage).GetFields(System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static)
			.Where(f => f.FieldType == typeof(string))
			.Select(f => new { name = f.Name, value = (string)f.GetValue(null), constant = f.IsLiteral })
			.ToList();
		return JsonSerializer.Serialize(fields, new JsonSerializerOptions { WriteIndented = true, Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping });
	}
}
