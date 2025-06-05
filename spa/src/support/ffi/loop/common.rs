pub fn result_from(res: i32) -> std::io::Result<i32> {
    if res >= 0 {
        Ok(res)
    } else {
        Err(std::io::Error::from_raw_os_error(-res))
    }
}

pub fn from_result(res: std::io::Result<i32>) -> i32 {
    match res {
        Ok(r) => r,
        Err(e) => -e.raw_os_error().unwrap_or(-1),
    }
}
