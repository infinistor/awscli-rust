// S3Probe가 호출하는 나머지 S3Client 연산. 설정 객체는 요청 XML 본문 비교를 위해 실제 쓰는 값으로 채운다.
// Rust 쪽(`tests/parity/s3_client.rs`)이 같은 값을 만들어 같은 op 이름으로 호출한다.
using System;
using System.Collections.Generic;
using System.IO;
using Amazon.S3;
using Amazon.S3.Model;

static partial class Program
{
	// 파일 본문 규칙: i번째 바이트 = 'a' + i % 26. Rust 쪽도 같은 규칙으로 파일을 만든다.
	static string MakeFile(long size, string ext = "bin")
	{
		var path = Path.Combine(Path.GetTempPath(), $"s3probe-{Guid.NewGuid():N}.{ext}");
		var data = new byte[size];
		for (var i = 0; i < size; i++) data[i] = (byte)('a' + i % 26);
		File.WriteAllBytes(path, data);
		return path;
	}

	static object S3Ops(TestCore.Client.S3Client client, S3Case spec)
	{
		var b = spec.Bucket;
		var k = spec.Key;
		var tags = new List<Tag> { new() { Key = "project", Value = "alpha" }, new() { Key = "env", Value = "test & dev" } };
		var retainUntil = new DateTime(2030, 1, 2, 3, 4, 5, DateTimeKind.Utc);
		switch (spec.Op)
		{
			// ---- 버킷 ----
			case "put-bucket-acl-canned": return client.PutBucketAcl(b, S3CannedACL.PublicRead);
			case "put-bucket-acl-policy":
				return client.PutBucketAcl(b, accessControlPolicy: new S3AccessControlList
				{
					Owner = new Owner { Id = "owner-id", DisplayName = "owner" },
					Grants = [new S3Grant { Grantee = new S3Grantee { CanonicalUser = "user-id", DisplayName = "user" }, Permission = S3Permission.FULL_CONTROL }],
				});
			case "get-bucket-acl": return client.GetBucketAcl(b);
			case "put-object-acl-canned": return client.PutObjectAcl(b, k, S3CannedACL.Private);
			case "put-object-acl-policy":
				return client.PutObjectAcl(b, k, accessControlPolicy: new S3AccessControlList
				{
					Owner = new Owner { Id = "owner-id", DisplayName = "owner" },
					Grants = [new S3Grant { Grantee = new S3Grantee { URI = "http://acs.amazonaws.com/groups/global/AllUsers" }, Permission = S3Permission.READ }],
				});
			case "get-object-acl": return client.GetObjectAcl(b, k, "v1");
			case "list-directory-buckets": return client.ListDirectoryBuckets(10, "token-1");
			case "delete-bucket": return client.DeleteBucket(b);
			case "get-bucket-ownership-controls": return client.GetBucketOwnershipControls(b);
			case "put-bucket-ownership-controls": return client.PutBucketOwnershipControls(b, ObjectOwnership.BucketOwnerEnforced);
			case "delete-bucket-ownership-controls": return client.DeleteBucketOwnershipControls(b);
			case "get-bucket-location": return client.GetBucketLocation(b);
			case "put-bucket-logging":
				return client.PutBucketLogging(b, new S3BucketLoggingConfig { TargetBucketName = "log-bucket", TargetPrefix = "logs/" });
			case "get-bucket-logging": return client.GetBucketLogging(b);
			case "put-bucket-notification":
				return client.PutBucketNotification(b,
					[new TopicConfiguration { Id = "t1", Topic = "arn:aws:sns:us-east-1:123456789012:topic", Events = [EventType.ObjectCreatedAll] }],
					[new QueueConfiguration { Id = "q1", Queue = "arn:aws:sqs:us-east-1:123456789012:queue", Events = [EventType.ObjectRemovedAll] }],
					[new LambdaFunctionConfiguration { Id = "l1", FunctionArn = "arn:aws:lambda:us-east-1:123456789012:function:f", Events = [EventType.ObjectCreatedPut] }]);
			case "get-bucket-notification": return client.GetBucketNotification(b);
			case "put-bucket-versioning-suspended": return client.PutBucketVersioning(b, VersionStatus.Suspended);
			case "get-bucket-versioning": return client.GetBucketVersioning(b);
			case "put-cors":
				return client.PutCORS(b, new CORSConfiguration
				{
					Rules = [new CORSRule
					{
						Id = "rule1",
						AllowedMethods = ["GET", "PUT"],
						AllowedOrigins = ["https://example.com"],
						AllowedHeaders = ["*"],
						ExposeHeaders = ["ETag"],
						MaxAgeSeconds = 3000,
					}],
				});
			case "get-cors": return client.GetCORS(b);
			case "delete-cors": return client.DeleteCORS(b);
			case "get-bucket-tagging": return client.GetBucketTagging(b);
			case "put-bucket-tagging": return client.PutBucketTagging(b, tags);
			case "delete-bucket-tagging": return client.DeleteBucketTagging(b);
			case "put-lifecycle":
				return client.PutLifecycleConfiguration(b, new LifecycleConfiguration
				{
					Rules =
					[
						new LifecycleRule
						{
							Id = "expire-logs",
							Status = LifecycleRuleStatus.Enabled,
							Filter = new LifecycleFilter { LifecycleFilterPredicate = new LifecyclePrefixPredicate { Prefix = "logs/" } },
							Expiration = new LifecycleRuleExpiration { Days = 30 },
							Transitions = [new LifecycleTransition { Days = 10, StorageClass = S3StorageClass.StandardInfrequentAccess }],
							NoncurrentVersionExpiration = new LifecycleRuleNoncurrentVersionExpiration { NoncurrentDays = 7 },
							AbortIncompleteMultipartUpload = new LifecycleRuleAbortIncompleteMultipartUpload { DaysAfterInitiation = 3 },
						},
					],
				});
			case "get-lifecycle": return client.GetLifecycleConfiguration(b);
			case "delete-lifecycle": return client.DeleteLifecycle(b);
			case "put-bucket-policy": return client.PutBucketPolicy(b, "{\"Version\":\"2012-10-17\",\"Statement\":[]}");
			case "get-bucket-policy": return client.GetBucketPolicy(b);
			case "delete-bucket-policy": return client.DeleteBucketPolicy(b);
			case "get-bucket-policy-status": return client.GetBucketPolicyStatus(b);
			case "put-object-lock-configuration":
				return client.PutObjectLockConfiguration(b, new ObjectLockConfiguration
				{
					ObjectLockEnabled = ObjectLockEnabled.Enabled,
					Rule = new ObjectLockRule { DefaultRetention = new DefaultRetention { Mode = ObjectLockRetentionMode.Governance, Days = 1 } },
				});
			case "get-object-lock-configuration": return client.GetObjectLockConfiguration(b);
			case "put-public-access-block":
				return client.PutPublicAccessBlock(b, new PublicAccessBlockConfiguration { BlockPublicAcls = true, IgnorePublicAcls = true, BlockPublicPolicy = false, RestrictPublicBuckets = false });
			case "get-public-access-block": return client.GetPublicAccessBlock(b);
			case "delete-public-access-block": return client.DeletePublicAccessBlock(b);
			case "get-bucket-encryption": return client.GetBucketEncryption(b);
			case "put-bucket-encryption":
				return client.PutBucketEncryption(b, new ServerSideEncryptionConfiguration
				{
					ServerSideEncryptionRules = [new ServerSideEncryptionRule
					{
						ServerSideEncryptionByDefault = new ServerSideEncryptionByDefault { ServerSideEncryptionAlgorithm = ServerSideEncryptionMethod.AES256 },
						BucketKeyEnabled = true,
					}],
				});
			case "delete-bucket-encryption": return client.DeleteBucketEncryption(b);
			case "get-bucket-website": return client.GetBucketWebsite(b);
			case "put-bucket-website":
				return client.PutBucketWebsite(b, new WebsiteConfiguration
				{
					IndexDocumentSuffix = "index.html",
					ErrorDocument = "error.html",
					RoutingRules = [new RoutingRule
					{
						Condition = new RoutingRuleCondition { KeyPrefixEquals = "docs/" },
						Redirect = new RoutingRuleRedirect { ReplaceKeyPrefixWith = "documents/" },
					}],
				});
			case "delete-bucket-website": return client.DeleteBucketWebsite(b);
			case "get-bucket-inventory": return client.GetBucketInventory(b, "inv1");
			case "list-bucket-inventory": return client.ListBucketInventory(b);
			case "put-bucket-inventory":
				return client.PutBucketInventory(b, new InventoryConfiguration
				{
					InventoryId = "inv1",
					IsEnabled = true,
					Destination = new InventoryDestination
					{
						S3BucketDestination = new InventoryS3BucketDestination { BucketName = "arn:aws:s3:::dest-bucket", Prefix = "inv/", InventoryFormat = InventoryFormat.CSV },
					},
					Schedule = new InventorySchedule { Frequency = InventoryFrequency.Daily },
					IncludedObjectVersions = InventoryIncludedObjectVersions.All,
					InventoryFilter = new InventoryFilter { InventoryFilterPredicate = new InventoryPrefixPredicate("data/") },
				});
			case "delete-bucket-inventory": return client.DeleteBucketInventory(b, "inv1");
			case "get-bucket-metrics": return client.GetBucketMetrics(b, "m1");
			case "list-bucket-metrics": return client.ListBucketMetrics(b);
			case "put-bucket-metrics":
				return client.PutBucketMetrics(b, new MetricsConfiguration
				{
					MetricsId = "m1",
					MetricsFilter = new MetricsFilter { MetricsFilterPredicate = new MetricsPrefixPredicate("data/") },
				});
			case "delete-bucket-metrics": return client.DeleteBucketMetrics(b, "m1");
			case "get-bucket-analytics": return client.GetBucketAnalytics(b, "a1");
			case "list-bucket-analytics": return client.ListBucketAnalytics(b);
			case "put-bucket-analytics":
				return client.PutBucketAnalytics(b, new AnalyticsConfiguration
				{
					AnalyticsId = "a1",
					AnalyticsFilter = new AnalyticsFilter { AnalyticsFilterPredicate = new AnalyticsPrefixPredicate("data/") },
					StorageClassAnalysis = new StorageClassAnalysis
					{
						DataExport = new StorageClassAnalysisDataExport
						{
							OutputSchemaVersion = StorageClassAnalysisSchemaVersion.V_1,
							Destination = new AnalyticsExportDestination
							{
								S3BucketDestination = new AnalyticsS3BucketDestination { BucketName = "arn:aws:s3:::dest-bucket", Prefix = "analytics/", Format = AnalyticsS3ExportFileFormat.CSV },
							},
						},
					},
				});
			case "delete-bucket-analytics": return client.DeleteBucketAnalytics(b, "a1");

			// ---- 객체 ----
			case "put-object-tagging-header": return client.PutObject(b, k, body: "hello world", tagSet: tags);
			case "put-object-file":
				{
					var path = MakeFile(spec.FileSize > 0 ? spec.FileSize : 32, spec.Ext);
					try { return client.PutObject(b, k, filePath: path, useChunkEncoding: spec.Chunked); } finally { File.Delete(path); }
				}
			case "put-object-stream": return client.PutObject(b, k, inputStream: new MemoryStream(System.Text.Encoding.UTF8.GetBytes(spec.Body)), useChunkEncoding: spec.Chunked);
			case "copy-object": return client.CopyObject("src-bucket", "src/key 1.txt", b, k, "v1");
			case "copy-object-special": return client.CopyObject("src-bucket", "src/한글 a+b(1)*'!~_-.txt", b, "dst/한글 key.txt", "v 1/+");
			case "list-versions": return client.ListVersions(b, "dir/", "km", "vm", 50, "/");
			case "get-object-tagging": return client.GetObjectTagging(b, k, "v1");
			case "put-object-tagging": return client.PutObjectTagging(b, k, new Tagging { TagSet = tags });
			case "delete-object-tagging": return client.DeleteObjectTagging(b, k);
			case "get-object-retention": return client.GetObjectRetention(b, k, "v1");
			case "put-object-retention":
				return client.PutObjectRetention(b, k, new ObjectLockRetention { Mode = ObjectLockRetentionMode.Governance, RetainUntilDate = retainUntil }, versionId: "v1", bypass: true);
			case "put-object-retention-nobypass":
				return client.PutObjectRetention(b, k, new ObjectLockRetention { Mode = ObjectLockRetentionMode.Compliance, RetainUntilDate = retainUntil });
			case "put-object-legal-hold": return client.PutObjectLegalHold(b, k, new ObjectLockLegalHold { Status = ObjectLockLegalHoldStatus.On }, "v1");
			case "get-object-legal-hold": return client.GetObjectLegalHold(b, k, "v1");
			case "get-bucket-replication": return client.GetBucketReplication(b);
			case "put-bucket-replication":
				return client.PutBucketReplication(b, new ReplicationConfiguration
				{
					Role = "arn:aws:iam::123456789012:role/replication",
					Rules =
					[
						new ReplicationRule
						{
							Id = "rule1",
							Status = ReplicationRuleStatus.Enabled,
							Priority = 1,
							Filter = new ReplicationRuleFilter { Prefix = "docs/" },
							DeleteMarkerReplication = new DeleteMarkerReplication { Status = DeleteMarkerReplicationStatus.Disabled },
							Destination = new ReplicationDestination { BucketArn = "arn:aws:s3:::dest-bucket", StorageClass = S3StorageClass.StandardInfrequentAccess },
						},
					],
				}, "token-ignored");
			case "delete-bucket-replication": return client.DeleteBucketReplication(b);
			case "restore-object": return client.RestoreObject(b, k, "v1", 7);
			case "restore-object-nodays": return client.RestoreObject(b, k);

			// ---- 멀티파트 ----
			case "initiate-multipart-upload": return client.InitiateMultipartUpload(b, k);
			case "upload-part-file":
				{
					var path = MakeFile(32);
					try { return client.UploadPart(b, k, "upload-1", 2, filePath: path, filePosition: 4, partSize: 8, useChunkEncoding: spec.Chunked); } finally { File.Delete(path); }
				}
			case "copy-part": return client.CopyPart("src-bucket", "src/key.txt", b, k, "upload-1", 3, 0, 1023, "v1");
			case "complete-multipart-upload":
				return client.CompleteMultipartUpload(b, k, "upload-1", [new PartETag(1, "\"etag1\""), new PartETag(2, "\"etag2\"")]);
			case "abort-multipart-upload": return client.AbortMultipartUpload(b, k, "upload-1");
			case "list-multipart-uploads": return client.ListMultipartUploads(b, "dir/", "um", "km", 50, "/");
			case "list-parts": return client.ListParts(b, k, "upload-1", 2, 50);
			case "list-parts-nomarker": return client.ListParts(b, k, "upload-1");

			// ---- TransferUtility ----
			case "upload":
				{
					var path = MakeFile(spec.FileSize, spec.Ext);
					try { client.Upload(b, k, path, spec.PartSize, spec.ThreadCount); } finally { File.Delete(path); }
					return "done";
				}
			case "upload-content-type":
				{
					var path = MakeFile(spec.FileSize, spec.Ext);
					try { client.Upload(b, k, path, spec.PartSize, spec.ThreadCount, contentType: "text/csv"); } finally { File.Delete(path); }
					return "done";
				}
			case "upload-bytes":
				{
					var data = new byte[spec.FileSize];
					for (var i = 0; i < data.Length; i++) data[i] = (byte)('a' + i % 26);
					client.Upload(b, k, null, spec.PartSize, spec.ThreadCount, byteBody: data);
					return "done";
				}
			case "upload-bytes-and-file":
				{
					var data = new byte[spec.FileSize];
					for (var i = 0; i < data.Length; i++) data[i] = (byte)('a' + i % 26);
					var path = MakeFile(5, spec.Ext);
					try { client.Upload(b, k, path, spec.PartSize, spec.ThreadCount, byteBody: data); } finally { File.Delete(path); }
					return "done";
				}
			case "upload-stream":
				{
					var data = new byte[spec.FileSize];
					for (var i = 0; i < data.Length; i++) data[i] = (byte)('a' + i % 26);
					client.Upload(b, k, null, spec.PartSize, spec.ThreadCount, body: new MemoryStream(data), contentType: spec.Ext == "none" ? null : "text/csv");
					return "done";
				}
			case "download":
				{
					var path = Path.Combine(Path.GetTempPath(), $"s3probe-{Guid.NewGuid():N}.out");
					try
					{
						client.Download(b, k, path, "v1");
						return new { downloaded = File.ReadAllText(path) };
					}
					finally { File.Delete(path); }
				}

			case "download-existing":
				{
					var path = Path.Combine(Path.GetTempPath(), $"s3probe-{Guid.NewGuid():N}.out");
					File.WriteAllText(path, "OLDOLDOLDOLDOLDOLD");
					try
					{
						client.Download(b, k, path);
						return new { downloaded = File.ReadAllText(path) };
					}
					finally { File.Delete(path); }
				}
			case "download-newdir":
				{
					var dir = Path.Combine(Path.GetTempPath(), $"s3probe-{Guid.NewGuid():N}");
					var path = Path.Combine(dir, "sub", "x.out");
					try
					{
						client.Download(b, k, path);
						return new { downloaded = File.ReadAllText(path) };
					}
					finally { if (Directory.Exists(dir)) Directory.Delete(dir, true); }
				}
			case "download-error-existing":
				{
					var path = Path.Combine(Path.GetTempPath(), $"s3probe-{Guid.NewGuid():N}.out");
					File.WriteAllText(path, "OLDOLDOLDOLDOLDOLD");
					try
					{
						try { client.Download(b, k, path); }
						catch (Exception e) { return new { error = e.GetType().FullName, message = e.Message, content = File.ReadAllText(path) }; }
						return "no error";
					}
					finally { File.Delete(path); }
				}

			// ---- 서명된 URL ----
			case "presign-get": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(3600), HttpVerb.GET);
			case "presign-put": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(900), HttpVerb.PUT, ServerSideEncryptionMethod.AES256, "text/plain");
			case "presign-delete": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(60), HttpVerb.DELETE);
			case "presign-frac-a": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddMilliseconds(3600400), HttpVerb.GET);
			case "presign-frac-b": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddMilliseconds(3599600), HttpVerb.GET);
			case "presign-local": return client.GeneratePresignedURL(b, k, DateTime.Now.AddSeconds(1800), HttpVerb.GET);
			case "presign-unspecified": return client.GeneratePresignedURL(b, k, DateTime.SpecifyKind(DateTime.UtcNow.AddSeconds(1800), DateTimeKind.Unspecified), HttpVerb.GET);
			case "presign-past": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(-30), HttpVerb.GET);
			case "presign-long": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddDays(8), HttpVerb.GET);
			case "presign-get-sse": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(3600), HttpVerb.GET, ServerSideEncryptionMethod.AES256);
			// 만료가 7일을 넘으면 .NET은 서명 V2 쿼리 형식을 쓴다. 고정 시각이라 서명까지 비교할 수 있다.
			case "presign-v2-get": return client.GeneratePresignedURL(b, k, new DateTime(2030, 1, 2, 3, 4, 5, DateTimeKind.Utc), HttpVerb.GET);
			case "presign-v2-put": return client.GeneratePresignedURL(b, k, new DateTime(2030, 1, 2, 3, 4, 5, DateTimeKind.Utc), HttpVerb.PUT, ServerSideEncryptionMethod.AES256, "text/plain");
			case "presign-v2-special-key": return client.GeneratePresignedURL(b, "dir/한글 a+b.txt", new DateTime(2030, 1, 2, 3, 4, 5, DateTimeKind.Utc), HttpVerb.DELETE);
			case "presign-special-key": return client.GeneratePresignedURL(b, "dir/한글 a+b.txt", DateTime.UtcNow.AddSeconds(3600), HttpVerb.GET);
			case "presign-head": return client.GeneratePresignedURL(b, k, DateTime.UtcNow.AddSeconds(3600), HttpVerb.HEAD);
			default: throw new ArgumentException(spec.Op);
		}
	}
}
