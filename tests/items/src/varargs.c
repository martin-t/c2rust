#include <stdarg.h>
#include <stdio.h>
#include <math.h>

// Can we correctly call an extern varargs function
void call_printf(void) {
    printf("%d, %f\n", 10, 1.5);
}

void my_vprintf(const char *format, va_list ap) {
  vprintf(format, ap);
}

void call_vprintf(const char *format, ...) {
  va_list ap;
  va_start(ap, format);
  my_vprintf(format, ap);
  va_end(ap);
}

// Simplified version of printf
void my_printf(const char *fmt, ...) {
  va_list ap;

  va_start(ap, fmt);
  while (*fmt) {
    switch (*fmt) {
    case '%':
      fmt++;
      if (!*fmt)
        break;

      switch (*fmt) {
      case 'i':
      case 'd':
        printf("%d", va_arg(ap, int));
        break;
      case 'f':
        printf("%f", va_arg(ap, double));
        break;
      case 's':
        printf("%s", va_arg(ap, char*));
        break;
      }

      break;

    default:
      putchar(*fmt);
      break;
    }

    fmt++;
  }

  va_end(ap);
}

void simple_vacopy(const char *fmt, ...) {
  va_list ap, aq;

  va_start(ap, fmt);
  va_copy(aq, ap);
  vprintf(fmt, ap);
  vprintf(fmt, aq);
  va_end(aq);
  va_end(ap);
}

// mirrors pattern from json-c's sprintbuf
void restart_valist(const char *fmt, ...) {
    va_list ap;
    // start
    va_start(ap, fmt);
    vprintf(fmt, ap);
    va_end(ap);
    // restart
    va_start(ap, fmt);
    vprintf(fmt, ap);
    va_end(ap);
}

// From: https://en.cppreference.com/w/c/variadic/va_copy (CC-BY-SA)
double sample_stddev(int count, ...)
{
    /* Compute the mean with args1. */
    double sum = 0;
    va_list args1;
    va_start(args1, count);
    va_list args2;
    va_copy(args2, args1);   /* copy va_list object */
    for (int i = 0; i < count; ++i) {
        double num = va_arg(args1, double);
        sum += num;
    }
    va_end(args1);
    double mean = sum / count;

    /* Compute standard deviation with args2 and mean. */
    double sum_sq_diff = 0;
    for (int i = 0; i < count; ++i) {
        double num = va_arg(args2, double);
        sum_sq_diff += (num-mean) * (num-mean);
    }
    va_end(args2);
    return sqrt(sum_sq_diff / count);
}
