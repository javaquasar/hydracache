# Isolate the received descriptor lifetime oracle from unrelated forks

MSRV passes all real loss/control/report cases but fails the new pipe EOF check:
the read returns EAGAIN after receiver refusal. The ordinary libtest process runs
other fork/exec tests concurrently. CLOEXEC does not prevent a writer description
from being inherited between their fork and exec/close_range. A transient foreign
setup child can therefore keep the pipe open without any received-FD leak. The
failure remains retained; do not change EOF into a sleep or timing tolerance.

Move only that oracle into a third closed `fd-lifetime` role of the same exact
libtest entry, with the same cleared environment, original subprocess pidfd,
owned process group,30-second outer bound and8192-byte output bound. Its pipe is
created after exec inside that dedicated process and it performs no further fork
or process launch. Kernel credentials and SCM_RIGHTS still cross the actual local
socketpair, refusal/drop must still close every received writer, and EOF must be
immediate with unchanged assertions. Positive control still observes EAGAIN while
its received report owns writers and EOF only after report drop.

Only fixed private libtest roles are supported; no caller command/PID/path/FD,
normal-library consumer or production launcher is added. Do not serialize away
the four independent actual monitors or weaken any real death/progress checks.
Retain the earlier concurrent EOF failure and distinguish it from a proved leak.
