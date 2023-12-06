BUILD_MODE ?= debug

CC   := gcc

CSTD       := -std=c11
ifeq ($(BUILD_MODE), debug)
	CFLAGS ?= -O0 -g3
else
	CFLAGS ?= -Os
endif
IINC       := -I c_bindings
WARNINGS   := -Wall -Wextra -Wshadow -Werror


C_BINDINGS_TESTS := $(wildcard c_bindings_tests/*.c)
C_BINDINGS_ELFS  := $(C_BINDINGS_TESTS:.c=.elf)

all: $(C_BINDINGS_ELFS)

clean:
	$(RM) -rf $(C_BINDINGS_ELFS) c_bindings_tests/*.elf

.PHONY: all clean
.DEFAULT_GOAL := all


%.elf: %.c $(LIB)
	$(CC) $(CSTD) $(CFLAGS) $(IINC) $(WARNINGS) -o $@ $< -L target/$(BUILD_MODE) -Wl,-Bstatic -l crunch64 -Wl,-Bdynamic

# Print target for debugging
print-% : ; $(info $* is a $(flavor $*) variable set to [$($*)]) @true
