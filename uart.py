import serial
import time

# Replace '/dev/ttyS0' with your serial port and 9600 with your baud rate
ser = serial.Serial('/dev/ttyS0', 115000, timeout=1)

def send_message(message):
    """Send a message to the UART device."""
    ser.write(message.encode())
    print("Message sent:", message)

def receive_message():
    """Receive a message from the UART device."""
    incoming_data = ser.readline().decode().strip()
    print("Message received:", incoming_data)
    return incoming_data

try:
    # Example usage
    send_message("Hello, UART Device!")
    time.sleep(2)  # Wait for the device to respond
    received_msg = receive_message()

except KeyboardInterrupt:
    print("Program terminated")

finally:
    ser.close()
