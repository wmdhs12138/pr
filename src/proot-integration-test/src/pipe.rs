use super::TestResult;

pub fn probe() -> bool {
    true
}

pub fn test_pipe_baseline() -> TestResult {
    let mut fds: [libc::c_int; 2] = [-1, -1];
    let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if ret != 0 {
        return Err(format!("pipe() failed: errno={}", std::io::Error::last_os_error()));
    }
    unsafe {
        libc::close(fds[0]);
        libc::close(fds[1]);
    }
    Ok(())
}

pub fn test_pipe2_cloexec() -> TestResult {
    let mut fds: [libc::c_int; 2] = [-1, -1];
    let ret = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("pipe2(O_CLOEXEC) failed: {} (errno={})", err, err.raw_os_error().unwrap_or(0)));
    }
    unsafe {
        libc::close(fds[0]);
        libc::close(fds[1]);
    }
    Ok(())
}

pub fn test_pipe2_nonblock() -> TestResult {
    let mut fds: [libc::c_int; 2] = [-1, -1];
    let ret = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("pipe2(O_NONBLOCK) failed: {} (errno={})", err, err.raw_os_error().unwrap_or(0)));
    }
    unsafe {
        libc::close(fds[0]);
        libc::close(fds[1]);
    }
    Ok(())
}
