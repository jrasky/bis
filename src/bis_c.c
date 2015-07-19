#include <termios.h>
#include <unistd.h>
#include <string.h>

struct bis_error_info_t {
  char *error_str;
  char is_errno;
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
  if (tcsetattr(STDOUT_FILENO, TCSAFLUSH, &bis_term_info) != 0) {
    bis_error_info.error_str = "Error restoring terminal attributes";
    bis_error_info.is_errno = 1;
    return -1;
  }

  // return success
  return 0;
}
