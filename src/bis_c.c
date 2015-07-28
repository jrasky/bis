#include <termios.h>
#include <unistd.h>
#include <string.h>
#include <sys/ioctl.h>
#include <signal.h>
#include <errno.h>

struct bis_error_info_t {
  char *error_str;
  char is_errno;
};

struct bis_term_size_t {
  unsigned short rows;
  unsigned short cols;
};

static char bis_term_info_set = 0;
static struct termios bis_term_info;

struct bis_error_info_t bis_error_info = {
  .error_str = (char *) 0,
  .is_errno = 0
};

int bis_prepare_terminal() {
  struct termios terminfo_p;
  // get terminal options
  if (tcgetattr(STDOUT_FILENO, &terminfo_p) != 0) {
    bis_error_info.error_str = "Error getting terminal attributes";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // copy to the global object
  memcpy(&bis_term_info, &terminfo_p, sizeof(struct termios));

  // update the info variable
  bis_term_info_set = 1;

  // disable canonical mode
  terminfo_p.c_lflag &= ~ICANON;

  // disable echo
  terminfo_p.c_lflag &= ~ECHO;

  // set terminal options
  if (tcsetattr(STDOUT_FILENO, TCSAFLUSH, &terminfo_p) != 0) {
    bis_error_info.error_str = "Error setting terminal attributes";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // return success
  return 0;
}

int bis_restore_terminal() {
  if (bis_term_info_set != 1) {
    bis_error_info.error_str = "bis_restore_terminal called before bis_prepare_terminal";
    bis_error_info.is_errno = 0;
    return -1;
  }

  // set terminal options
  if (tcsetattr(STDOUT_FILENO, TCSANOW, &bis_term_info) != 0) {
    bis_error_info.error_str = "Error restoring terminal attributes";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // return success
  return 0;
}

int bis_get_terminal_size(struct bis_term_size_t *size) {
  struct winsize term_size;
  // request the terminal size
  if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &term_size) != 0) {
    bis_error_info.error_str = "ioctl call failed";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // put the info into size
  size->rows = term_size.ws_row;
  size->cols = term_size.ws_col;

  // return success
  return 0;
}

int bis_mask_sigint() {
  sigset_t set;

  sigemptyset(&set);
  sigaddset(&set, SIGINT);
  if (sigprocmask(SIG_BLOCK, &set, NULL) != 0) {
    bis_error_info.error_str = "sigprocmask failed";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // return success
  return 0;
}

int bis_wait_sigint() {
  sigset_t set;

  sigemptyset(&set);
  sigaddset(&set, SIGINT);

  int result;

  for (;;) {
    if ((result = sigwaitinfo(&set, NULL)) == -1) {
      if (errno != EINTR) {
        bis_error_info.error_str = "sigwaitinfo failed";
        bis_error_info.is_errno = 1;
        return -1;
      }

      // otherwise try again
    } else if (result != SIGINT) {
      // we caught some other signal
      bis_error_info.error_str = "Caught signal other than SIGINT";
      bis_error_info.is_errno = 0;
      return -1;
    } else {
      // we caught the signal we wanted
      return 0;
    }
  }
}

int bis_insert_input(const char *input) {
  // insert the input string into the input queue
  for (; *input != 0; input++) {
    if (ioctl(STDIN_FILENO, TIOCSTI, input) != 0) {
      bis_error_info.error_str = "ioctl call failed";
      bis_error_info.is_errno = 1;
      return -1;
    }
  }

  // returnt success
  return 0;
}
