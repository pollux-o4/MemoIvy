#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>

int main(void) {
    if (fork() != 0) return 0;
    setsid();
    int log = open(USER_HOME "/.local/share/memoivy-launch.log", O_WRONLY | O_CREAT | O_APPEND, 0600);
    if (log >= 0) { dup2(log, STDERR_FILENO); close(log); }
    pid_t app = fork();
    if (app == 0) {
        if (chdir(USER_HOME) != 0) { perror("chdir"); _exit(127); }
        execl(USER_HOME "/.local/bin/memoivy", "memoivy", (char *)NULL);
        perror("memoivy");
        _exit(127);
    }
    int status;
    if (app < 0 || waitpid(app, &status, 0) < 0 || !WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        execl("/usr/bin/zenity", "zenity", "--error", "--text=MemoIvy 실행에 실패했습니다. ~/.local/share/memoivy-launch.log를 확인하세요.", (char *)NULL);
    }
    return 0;
}
