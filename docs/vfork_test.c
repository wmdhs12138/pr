#include <unistd.h>
#include <stdio.h>
#include <sys/wait.h>

int main() {
    pid_t pid = vfork();
    if (pid == 0) {
        char *argv[] = {"cc", "/tmp/test.c", "-o", "/tmp/test_vf", NULL};
        char *envp[] = {NULL};
        execve("/usr/bin/cc", argv, envp);
        _exit(127);
    }
    int st;
    waitpid(pid, &st, 0);
    printf("vfork+execve: status=%d\n", WEXITSTATUS(st));
    return 0;
}
