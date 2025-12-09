# Playground Makefile - Test area for Orca

CC = gcc
CFLAGS = -Wall -Wextra -std=c11 -O2
TARGET = playground
SRC = main.c

.PHONY: all clean run

# Default target
all: $(TARGET)

# Build the executable
$(TARGET): $(SRC)
	$(CC) $(CFLAGS) -o $(TARGET) $(SRC)

# Run the program
run: $(TARGET)
	./$(TARGET)

# Clean build artifacts
clean:
	rm -f $(TARGET)
	@echo "Clean complete."

# Help target
help:
	@echo "Playground Makefile for Orca Testing"
	@echo ""
	@echo "Available targets:"
	@echo "  make          - Build the playground executable"
	@echo "  make all      - Same as 'make'"
	@echo "  make run      - Build and run the program"
	@echo "  make clean    - Remove built artifacts"
	@echo "  make help     - Show this help message"
