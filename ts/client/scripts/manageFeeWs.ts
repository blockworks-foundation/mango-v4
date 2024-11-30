import WebSocket from 'ws';

export function manageFeeWebSocket(
  wsUrl: string,
  rollingWindowSize: number = 120,
  onMeanCalculated: (mean: number | null) => void,
): () => void {
  let ws: WebSocket | null = null;
  const recentValues: number[] = [];

  function calculateRollingMean(values: number[]): number | null {
    if (values.length === 0) return null;
    const sum = values.reduce((acc, val) => acc + val, 0);
    return Math.floor(sum / values.length);
  }

  function connectWebSocket(): void {
    try {
      ws = new WebSocket(wsUrl);

      ws.addEventListener('open', () => {
        try {
          // console.log('Fee WebSocket opened');
          const message = JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method: 'blockPrioritizationFeesSubscribe',
            interval: 30,
          });
          ws?.send(message);
        } catch (error) {
          console.error('Error in open event:', error);
          onMeanCalculated(null);
        }
      });

      ws.addEventListener('close', () => {
        // console.log('Fee WebSocket closed');
        ws = null;
        // Attempt to reconnect after a delay
        setTimeout(connectWebSocket, 5000);
      });

      ws.addEventListener('error', (error) => {
        // console.log('Fee WebSocket error:', error);
        onMeanCalculated(null);
        ws?.close();
      });

      ws.addEventListener(
        'message',
        (event: { data: any; type: string; target: WebSocket }): void => {
          try {
            const parsedData = JSON.parse(event.data as string);
            const value = parsedData?.params?.result?.value.by_tx[15];

            if (value !== undefined && typeof value === 'number') {
              recentValues.push(value);
              if (recentValues.length > rollingWindowSize) {
                recentValues.shift();
              }

              const rollingMean = calculateRollingMean(recentValues);
              onMeanCalculated(rollingMean);
            }
          } catch (error) {
            console.error('Error processing message:', error);
            onMeanCalculated(-1);
          }
        },
      );
    } catch (error) {
      console.error('Error in connectWebSocket:', error);
      onMeanCalculated(-1);
      // Attempt to reconnect after a delay
      setTimeout(connectWebSocket, 5000);
    }
  }

  // Start the WebSocket connection
  connectWebSocket();

  // Return a function to close the WebSocket
  return () => {
    if (ws) {
      try {
        ws.close();
      } catch (error) {
        console.error('Error closing WebSocket:', error);
      }
    }
  };
}
