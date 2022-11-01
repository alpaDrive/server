# server
The main web server built on Actix Web

## Routes
1. Signup
    * Request type: POST
    * Format: JSON
        ```json
        {
            "name": "testuser2",
            "username": "testname2",
            "password": "testpass1",
            "uid": "testuid1",
            "email": "testemail2"
        }
        ```
    * Returns: The uid of the newly signed up user in MongoDB's ObjectID format
    
        ```json
        {
            "uid": {
                "$oid": "<user-ID>"
            }
        }
        ```

2. Login
    * Request type: GET
    * Format: JSON

        ```json
        {
            "username": "<username>",
            // "email": "<email>"
            "password": "<password>"
        }
        ```
    * Returns: An error if either the user doesn't exist or if the credentials are incorrect. Either one of username or email are required to complete the request. Or else the info of that specific user is returned
        ```json
        {
            "_id": {
                "$oid": "63307b31d0c18856548cef9d"
            },
            "name": "\"testuser2\"",
            "username": "\"testname2\"",
            "email": "\"testemail2\"",
            "vehicles": []
        }
        ```

3. Status
    * Request type: GET
    * Format: JSON
        ```json
        {
            "systemstat": false
        }
        ```
    * Returns: Active websocket connections indicating number of users and number of vehicles. Also provides system info if `systemstat: true`

        ```json
        {
            "active_users": 0,
            "active_vehicles": 0,
            "memory_available": "7.84 GB",
            "memory_used": "2.01 GB",
            "total_swap": "2.10 GB",
            "swap_used": "0.00 GB"
        }
        ```
4. Register a vehicle
    * Request type: POST
    * Format: JSON
    
        ```json
        {
            "company": "BMW",
            "model": "530i M sport"
        }
        ```
    * Returns: The MongoDB `ObjectId()` of the document related to the vehicle. This ID can be used later on for reference in the stack
        ```json
        {
            "success": "Vehicle was registered",
            "id": {
                "$oid": "6337feae9e332e5b3ad192b7"
            }
        }
        ```
5. Create and join a vehicle room
    * Request type: GET
    * Format: Plain URL route

        ```
        https://url.com/join/vehicle/{uid}
        ```
        where `uid` is the `$oid` of the vehicle generated during registration
    * Returns: A websocket connection upgrade to the room. The vehicle is in control of the room.
    * Notes: Only one instance of a vehicle should connect at a time. More than one instance of the same vehicle should never attempt to connect and make a room. If this happens, the server may lose contact with the existing instance and will also never connect to the new instance.

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