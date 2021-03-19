#include <stdio.h>

int main() {
  printf("beginning of program\n");
  void *ptr = alligator_alloc(10);
  printf("allocated address\n");
  alligator_dealloc(ptr);
  printf("freed address\n");
}
