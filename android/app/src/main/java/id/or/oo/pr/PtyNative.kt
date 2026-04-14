package id.or.oo.pr

object PtyNative {
    init {
        System.loadLibrary("ptyjni")
    }

    fun forkPty(cmd: String, args: Array<String>?, envVars: Array<String>?, rows: Int, cols: Int): Int {
        return nativeForkPty(cmd, args, envVars, rows, cols)
    }

    fun read(fd: Int, buf: ByteArray, offset: Int, length: Int): Int {
        return nativeRead(fd, buf, offset, length)
    }

    fun write(fd: Int, buf: ByteArray, offset: Int, length: Int): Int {
        return nativeWrite(fd, buf, offset, length)
    }

    fun resize(fd: Int, rows: Int, cols: Int): Int {
        return nativeResize(fd, rows, cols)
    }

    fun waitPid(pid: Int): Int {
        return nativeWaitPid(pid)
    }

    fun close(fd: Int) {
        nativeClose(fd)
    }

    fun getPid(): Int {
        return nativeGetPid()
    }

    private external fun nativeForkPty(cmd: String, args: Array<String>?, envVars: Array<String>?, rows: Int, cols: Int): Int
    private external fun nativeRead(fd: Int, buf: ByteArray, offset: Int, length: Int): Int
    private external fun nativeWrite(fd: Int, buf: ByteArray, offset: Int, length: Int): Int
    private external fun nativeResize(fd: Int, rows: Int, cols: Int): Int
    private external fun nativeWaitPid(pid: Int): Int
    private external fun nativeClose(fd: Int)
    private external fun nativeGetPid(): Int
}
