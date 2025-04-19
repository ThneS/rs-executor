#!/bin/bash
for i in {1..20}; do
  curl -X POST -H "Content-Type: application/json" -d '{
    "Tasks":"21"
  }' http://localhost:40000/start &
done
wait

# chmod +x test.sh && ./test.sh