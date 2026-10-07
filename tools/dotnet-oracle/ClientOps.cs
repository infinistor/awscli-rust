// PortalManager·MoverClient·ZeroMqClient 기준 출력. 실제 TestCore 클래스를 로컬 서버에 연결해 요청을 기록한다.
// Rust 쪽(`tests/parity/portal.rs`, `mover.rs`, `zeromq.rs`)이 같은 사례 파일로 같은 동작을 비교한다.
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using NetMQ;

static partial class Program
{
	public sealed class ClientCase
	{
		public string Op { get; set; }
		public string ApiKey { get; set; } = "test-api-key";
		// PortalConfig.URL에 이어 붙일 문자열(끝의 '/' 제거 확인용).
		public string UrlSuffix { get; set; } = "";
		public string Volume { get; set; } = "vol1";
		public string User { get; set; } = "user1";
		public string Password { get; set; } = "pw";
		public string Ip { get; set; } = "10.0.0.1";
		public string Bucket { get; set; }
		public ulong Size { get; set; } = 1000000000;
		public int JobId { get; set; }
		public JsonElement? Request { get; set; }
		public int Status { get; set; } = 200;
		public string ResponseBody { get; set; } = "";
		public Dictionary<string, string> ResponseHeaders { get; set; } = [];
		public List<S3Route> Routes { get; set; } = [];
		// 응답 본문만 바꿔가며 같은 연산을 반복한다(JSON 읽기 경계 사례용). 결과·오류·로그만 기록한다.
		public List<string> Variants { get; set; } = [];
		// ZeroMQ: 서버가 돌려줄 응답 문자열.
		public string Reply { get; set; } = "OK!";
		public string ServiceType { get; set; } = "svc";
	}

	// log4net 로그를 메모리에 모은다(TestCore 어셈블리 저장소).
	static log4net.Appender.MemoryAppender StartLogCapture()
	{
		var appender = new log4net.Appender.MemoryAppender();
		var repository = log4net.LogManager.GetRepository(typeof(TestCore.Portal.PortalManager).Assembly);
		log4net.Config.BasicConfigurator.Configure(repository, appender);
		return appender;
	}

	static object[] CollectLogs(log4net.Appender.MemoryAppender appender) =>
		appender.GetEvents().Select(e => (object)new { level = e.Level.Name, message = e.RenderedMessage }).ToArray();

	// 사례의 응답을 돌려주는 로컬 서버를 띄운다(연결마다 같은 응답, Routes 지원).
	static (System.Net.Sockets.TcpListener listener, int port, System.Collections.Concurrent.ConcurrentQueue<object> requests, System.Threading.CancellationTokenSource cts) StartClientServer(ClientCase spec)
	{
		var s3 = new S3Case { Status = spec.Status, ResponseBody = spec.ResponseBody, ResponseHeaders = spec.ResponseHeaders, Routes = spec.Routes };
		var listener = new System.Net.Sockets.TcpListener(System.Net.IPAddress.Loopback, 0);
		listener.Start();
		var port = ((System.Net.IPEndPoint)listener.LocalEndpoint).Port;
		var requests = new System.Collections.Concurrent.ConcurrentQueue<object>();
		var cts = new System.Threading.CancellationTokenSource();
		System.Threading.Tasks.Task.Run(async () =>
		{
			while (!cts.IsCancellationRequested)
			{
				System.Net.Sockets.TcpClient socket;
				try { socket = await listener.AcceptTcpClientAsync(cts.Token); } catch { break; }
				_ = System.Threading.Tasks.Task.Run(() => ServeS3(socket, s3, requests));
			}
		});
		return (listener, port, requests, cts);
	}

	public sealed class CaseResult
	{
		[System.Text.Json.Serialization.JsonPropertyName("port")] public int Port { get; set; }
		[System.Text.Json.Serialization.JsonPropertyName("requests")] public object[] Requests { get; set; }
		[System.Text.Json.Serialization.JsonPropertyName("result")] public object Result { get; set; }
		[System.Text.Json.Serialization.JsonPropertyName("error")] public object Error { get; set; }
		[System.Text.Json.Serialization.JsonPropertyName("logs")] public object[] Logs { get; set; }
	}

	static CaseResult RunCase(ClientCase spec, Func<ClientCase, int, object> body, log4net.Appender.MemoryAppender logs)
	{
		logs.Clear();
		var (listener, port, requests, cts) = StartClientServer(spec);
		object result = null, error = null;
		try { result = body(spec, port); }
		catch (Exception e) { error = ErrorOf(e); }
		System.Threading.Thread.Sleep(100);
		cts.Cancel();
		listener.Stop();
		return new CaseResult { Port = port, Requests = requests.ToArray(), Result = result, Error = error, Logs = CollectLogs(logs) };
	}

	// 사례 하나를 실행한다. Variants가 있으면 응답 본문만 바꿔가며 반복하고 결과·오류·로그만 기록한다.
	static string RunClientCase(string casePath, Func<ClientCase, int, object> body)
	{
		var spec = ReadClientCase(casePath);
		var logs = StartLogCapture();
		if (spec.Variants.Count == 0) return JsonSerializer.Serialize(RunCase(spec, body, logs), Json);
		var variants = spec.Variants.Select(v =>
		{
			spec.ResponseBody = v;
			var r = RunCase(spec, body, logs);
			return new { responseBody = v, result = r.Result, error = r.Error, logs = r.Logs };
		}).ToArray();
		return JsonSerializer.Serialize(new { variants }, Json);
	}

	static object ErrorOf(Exception e) => new { type = e.GetType().FullName, message = e.Message };

	static ClientCase ReadClientCase(string path) =>
		JsonSerializer.Deserialize<ClientCase>(File.ReadAllText(path), new JsonSerializerOptions { PropertyNameCaseInsensitive = true });

	// 결과 객체는 원본 ToJsonString() 문자열로 기록한다.
	static string AsJson(object value) => value == null ? null : TestCore.JsonExtensions.ToJsonString(value);

	static string Portal(string casePath) => RunClientCase(casePath, (spec, port) =>
	{
		var manager = new TestCore.Portal.PortalManager(new TestCore.Portal.PortalConfig($"http://127.0.0.1:{port}{spec.UrlSuffix}", spec.ApiKey));
		switch (spec.Op)
		{
			case "health": return manager.HealthCheck();
			case "get-volume": return AsJson(manager.GetVolume(spec.Volume));
			case "create-volume": manager.CreateVolume(spec.Volume, spec.Size, spec.Password); return null;
			case "start-volume": manager.StartVolume(spec.Volume); return null;
			case "stop-volume": manager.StopVolume(spec.Volume); return null;
			case "delete-volume": manager.DeleteVolume(spec.Volume); return null;
			case "assign-volume": manager.AssignVolume(spec.Volume, spec.User, spec.Size); return null;
			case "get-user": return AsJson(manager.GetUser(spec.User));
			case "is-user": return manager.IsUser(spec.User);
			case "create-user": manager.CreateUser(spec.Volume, spec.User, spec.Size, spec.Password); return null;
			case "get-user-credential": return AsJson(manager.GetUserCredential(spec.Volume, spec.User));
			case "delete-user": manager.DeleteUser(spec.User); return null;
			case "put-access-ip": manager.PutAccessIp(spec.Volume, spec.User, spec.Ip, spec.Bucket); return null;
			case "delete-access-ip": manager.DeleteAccessIp(spec.Volume, spec.User, spec.Bucket); return null;
			default: throw new ArgumentException(spec.Op);
		}
	});

	static string Mover(string casePath) => RunClientCase(casePath, (spec, port) =>
	{
		var client = new TestCore.Mover.MoverClient($"http://127.0.0.1:{port}{spec.UrlSuffix}");
		switch (spec.Op)
		{
			case "start":
				var request = JsonSerializer.Deserialize<TestCore.Mover.Request.RequestMoverStart>(spec.Request.Value.GetRawText());
				return client.MoverStart(request);
			case "status": return AsJson(client.MoverStatus(spec.User, spec.JobId));
			default: throw new ArgumentException(spec.Op);
		}
	});

	// ZeroMQ: NetMQ REP 서버를 같은 프로세스에서 띄워 ZeroMqClient가 보낸 프레임을 기록한다.
	static string ZeroMq(string casePath)
	{
		var spec = ReadClientCase(casePath);
		var logs = StartLogCapture();
		using var server = new NetMQ.Sockets.ResponseSocket();
		var port = server.BindRandomPort("tcp://127.0.0.1");
		string received = null;
		var task = System.Threading.Tasks.Task.Run(() =>
		{
			received = server.ReceiveFrameString();
			server.SendFrame(spec.Reply);
		});
		object result = null, error = null;
		try
		{
			result = spec.Op switch
			{
				"pause" => TestCore.Client.ZeroMqClient.Pause(spec.ServiceType, "127.0.0.1", port),
				"resume" => TestCore.Client.ZeroMqClient.Resume(spec.ServiceType, "127.0.0.1", port),
				_ => throw new ArgumentException(spec.Op),
			};
		}
		catch (Exception e) { error = ErrorOf(e); }
		task.Wait(TimeSpan.FromSeconds(5));
		return JsonSerializer.Serialize(new { port, received, result, error, logs = CollectLogs(logs) }, Json);
	}

	// 상호 운용 확인용: NetMQ REP 서버가 포트를 출력하고 요청 하나를 받아 응답한다. Rust 클라이언트를 붙여 본다.
	static string ZeroMqServe(string reply)
	{
		using var server = new NetMQ.Sockets.ResponseSocket();
		var port = server.BindRandomPort("tcp://127.0.0.1");
		Console.WriteLine($"PORT {port}");
		Console.Out.Flush();
		var received = server.ReceiveFrameString();
		server.SendFrame(reply);
		return JsonSerializer.Serialize(new { received }, Json);
	}

	// 상호 운용 확인용: ZeroMqClient(NetMQ REQ)로 지정한 서버에 요청한다. Rust REP 서버를 붙여 본다.
	static string ZeroMqCall(string op, string serviceType, string ip, int port)
	{
		var logs = StartLogCapture();
		var result = op == "pause" ? TestCore.Client.ZeroMqClient.Pause(serviceType, ip, port) : TestCore.Client.ZeroMqClient.Resume(serviceType, ip, port);
		return JsonSerializer.Serialize(new { result, logs = CollectLogs(logs) }, Json);
	}
}
