curl -X POST http://localhost:8080/set-rc \
     -H "Content-Type: application/json" \
     -d '{
           "roll": 1500,
           "pitch": 1500,
           "yaw": 1500,
           "thr": 1000,
           "aux1": 1000,
           "aux2": 1000,
           "aux3": 1000,
           "aux4": 1000
         }'