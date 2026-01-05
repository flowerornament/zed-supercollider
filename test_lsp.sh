#!/bin/bash
# Test LSP initialization to see what capabilities are advertised

# Create a named pipe for communication
PIPE=/tmp/lsp_test_$$
mkfifo $PIPE

# Start launcher in background, reading from pipe
./server/launcher/target/release/sc_launcher --mode lsp < $PIPE 2>&1 &
LSP_PID=$!

# Give sclang time to start
echo "Waiting for sclang to initialize..." >&2
sleep 8

# Send initialize request
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp","capabilities":{"textDocument":{"definition":{"dynamicRegistration":false},"completion":{"dynamicRegistration":false}}}}}'
LEN=${#INIT}

echo "Sending initialize request..." >&2
echo -e "Content-Length: $LEN\r\n\r\n$INIT" > $PIPE

# Wait for response
sleep 3

# Cleanup
kill $LSP_PID 2>/dev/null
rm -f $PIPE
echo "Done" >&2
