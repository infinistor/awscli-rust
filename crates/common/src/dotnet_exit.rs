//! .NET 런타임이 처리하지 않은 예외로 프로세스를 끝낼 때의 출력과 종료 코드.

/// 처리하지 않은 예외로 끝날 때의 종료 코드(`0xE0434352`).
pub const UNHANDLED_EXCEPTION_EXIT_CODE: i32 = 0xE043_4352_u32 as i32;
/// 처리하지 않은 `NullReferenceException`의 종료 코드(접근 위반 `0xC0000005`).
pub const NULL_REFERENCE_EXIT_CODE: i32 = 0xC000_0005_u32 as i32;
/// 처리하지 않은 `DivideByZeroException`의 종료 코드(정수 나눗셈 예외 `0xC0000094`).
pub const DIVIDE_BY_ZERO_EXIT_CODE: i32 = 0xC000_0094_u32 as i32;

/// 처리하지 않은 예외: .NET 런타임처럼 표준 오류에 알리고 비정상 종료 코드를 돌려준다.
pub fn unhandled(dotnet_type: &str, message: &str) -> i32 {
    eprintln!("Unhandled exception. {dotnet_type}: {message}");
    match dotnet_type {
        "System.NullReferenceException" => NULL_REFERENCE_EXIT_CODE,
        "System.DivideByZeroException" => DIVIDE_BY_ZERO_EXIT_CODE,
        _ => UNHANDLED_EXCEPTION_EXIT_CODE,
    }
}

/// 스레드 안에서 처리하지 않은 예외: 다른 스레드와 최종 처리를 기다리지 않고 프로세스를 바로 끝낸다.
pub fn crash(dotnet_type: &str, message: &str) -> ! {
    let code = unhandled(dotnet_type, message);
    std::process::exit(code)
}
