CC = g++
CFLAGS = -I../cpp -fPIC
LDFLAGS = -Wl,-rpath=../target/debug
LDLIBS = -ldl ../target/debug/libbambu_farm_client.so
TARGET = test

all: $(TARGET)

$(TARGET): test.o
	$(CC) $(LDFLAGS) -o $@ $^ $(LDLIBS)

debug: $(TARGET)
	gdb --args $(TARGET)

test.o: test.c
	$(CC) $(CFLAGS) -c $<

clean:
	rm -f $(TARGET) *.o

