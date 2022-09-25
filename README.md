# server
The main web server built on Actix Web

## Routes
1. Signup
    * Request type: POST
    * Format: JSON
        ```
        {
            "name": "testuser2",
            "username": "testname2",
            "password": "testpass1",
            "uid": "testuid1",
            "email": "testemail2"
        }
        ```
    * Returns: The uid of the newly signed up user in MongoDB's ObjectID format
    
        ```
        {
            "uid": {
                "$oid": "<user-ID>"
            }
        }
        ```

2. Status
    * Request type: GET
    * Format: JSON
        ```
        {
            "systemstat": false
        }
        ```
    * Returns: Active websocket connections indicating number of users and number of vehicles. Also provides system info if `systemstat: true`

        ```
        {
            "active_users": 0,
            "active_vehicles": 0,
            "memory_available": "7.84 GB",
            "memory_used": "2.01 GB",
            "total_swap": "2.10 GB",
            "swap_used": "0.00 GB"
        }
        ```

## Setup dev environment in WSL

* Install cargo
* Install MongoDB by following the guide [here](https://raw.githubusercontent.com/mongodb/mongo/master/debian/init.d).
* Start mongod on port 8080
    ```
    mongod --dbpath ~/data/db --port 8080
    ```
* Run the server
    ```
    cargo run
    ```