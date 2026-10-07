//! TESTCore `Util/TestCoreApplication.cs`: 인자 파싱, 설정 로딩, 명령 실행 흐름.

use std::time::Instant;

use awscli_rest_s3::{ChecksumAlgorithm, S3Client};
use tracing::{error, info};

use crate::bootstrap;
use crate::dispatch::{self, CommandContext};
use crate::menu::MenuList;
use crate::options::{self, ParseError};
use crate::version::version_info;

/// 원본 `ERROR_NORMAL`.
pub const ERROR_NORMAL: i32 = -1;
/// 원본 `ERROR_COMMAND_NOT_FOUND`.
pub const ERROR_COMMAND_NOT_FOUND: i32 = -127;
/// .NET 런타임이 처리하지 않은 예외로 끝날 때의 종료 코드(`0xE0434352`).
pub const UNHANDLED_EXCEPTION_EXIT_CODE: i32 = 0xE043_4352_u32 as i32;
/// 처리하지 않은 `NullReferenceException`의 종료 코드(접근 위반 `0xC0000005`).
pub const NULL_REFERENCE_EXIT_CODE: i32 = 0xC000_0005_u32 as i32;

/// 처리하지 않은 예외: .NET 런타임처럼 표준 오류에 알리고 비정상 종료 코드를 돌려준다.
fn unhandled(dotnet_type: &str, message: &str) -> i32 {
    eprintln!("Unhandled exception. {dotnet_type}: {message}");
    if dotnet_type == "System.NullReferenceException" {
        NULL_REFERENCE_EXIT_CODE
    } else {
        UNHANDLED_EXCEPTION_EXIT_CODE
    }
}

/// 명령행 인자(프로그램 이름 제외)로 애플리케이션을 실행하고 종료 코드를 돌려준다.
pub fn run(args: &[String]) -> i32 {
    let started = Instant::now();
    info!("Main start");

    let parsed = match options::parse(args) {
        Ok(parsed) => parsed,
        Err(ParseError::Option {
            message,
            option_name,
        }) => {
            println!("{message}");
            // 원본은 `e.OptionName.Replace(...)`를 그대로 불러, 이름이 없는 오류(묶음 오류)면 NullReferenceException이 난다.
            let Some(option_name) = option_name else {
                return unhandled(
                    "System.NullReferenceException",
                    "Object reference not set to an instance of an object.",
                );
            };
            let description = options::find_option(&option_name.replace("--", ""))
                .map_or("", |def| def.description);
            println!("{option_name} : {description}");
            return ERROR_COMMAND_NOT_FOUND;
        }
        Err(ParseError::Unhandled {
            dotnet_type,
            message,
        }) => return unhandled(dotnet_type, &message),
    };
    for option in &parsed.extra {
        println!("{option} : invalid.");
    }
    if !parsed.extra.is_empty() {
        return ERROR_NORMAL;
    }

    let mut command_options = parsed.options;
    if command_options.worker || command_options.controller {
        if command_options.help {
            print!("{}", options::write_option_descriptions());
            return 0;
        }
        // TODO(6단계): DistributedApplication 이식.
        eprintln!("분산 실행 오류: 아직 이식되지 않았습니다.");
        return ERROR_NORMAL;
    }
    if command_options.version {
        println!("{}", version_info());
        return 0;
    }

    let config = bootstrap::load(&mut command_options);
    if config.is_none() && !command_options.help {
        println!(
            "설정 파일을 불러오지 못했습니다: {}",
            command_options.config_path.as_deref().unwrap_or("")
        );
        return ERROR_NORMAL;
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio 런타임");
    let help = command_options.help;
    let result = runtime.block_on(async {
        let client = match (&config, help) {
            (Some(config), false) => Some(S3Client::from_user(
                &config.main_user,
                command_options.admin,
                config.main.retry_count,
                command_options.menu == MenuList::PutObject
                    && command_options.checksum_type != ChecksumAlgorithm::None,
            )),
            _ => None,
        };
        let mut context = CommandContext {
            config,
            options: command_options,
            client,
        };
        dispatch::execute(&mut context).await
    });
    let result = match result {
        Ok(result) => result,
        Err(e) => {
            error!("{e}");
            return ERROR_NORMAL;
        }
    };

    if !help {
        info!("Main complete time = {}ms", started.elapsed().as_millis());
    }
    result
}
