#include <stdio.h>
#include <stdlib.h>

int main() {
  void *mptr = malloc(4294967296);
  printf("malloc(4 GB %p\n", mptr);
  
  /* printf("beginning of program\n"); */
  /* void *ptr = alligator_alloc(10); */
  /* printf("allocated address\n"); */
  /* alligator_dealloc(ptr); */
  /* printf("freed address\n"); */
}
