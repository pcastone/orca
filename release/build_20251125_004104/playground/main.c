#include <stdio.h>
#include <stdlib.h>

int main(int argc, char *argv[]) {
    printf("=== Playground Test Area for Orca ===\n");
    printf("Hello from playground!\n");
    printf("This is a test environment for orca integration.\n\n");

    if (argc > 1) {
        printf("Arguments received:\n");
        for (int i = 1; i < argc; i++) {
            printf("  [%d]: %s\n", i, argv[i]);
        }
    } else {
        printf("No arguments provided.\n");
    }

    printf("\nPlayground test completed successfully.\n");
    return 0;
}
