# server
The main web server built on Actix Web

## Routes
Routes are mainly for starting a connection with the server. For instance, registering vehicles and users, creating, joining & leaving rooms, etc a.k.a the generic boring stuff. Sadly, we can't skip it. There is no magic that will manage the boring stuff for us.
1. ### Signup
    * Request type: POST
    * Route: `/signup`
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

2. ### Login
    * Request type: POST
    * Route: `/login`
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
            "uid": {
                "$oid": "63307b31d0c18856548cef9d"
            },
            "name": "\"testuser2\"",
            "username": "\"testname2\"",
            "email": "\"testemail2\"",
            "vehicles": []
        }
        ```

3. ### Status
    * Request type: POST
    * Route: `/status`
    * Format: JSON
        ```json
        {
            "systemstat": false
        }
        ```
    * Returns: Active websocket connections indicating number of users and number of vehicles. Also provides system info if `systemstat: true`. It shouls be noted that an average computer takes ~20ms to measure system resources. So the response time might be delayed with `systemstat: true`.

        ```json
        {
            "active_users": 0,
            "active_vehicles": 0,
            "active_sessions": 0,
            "memory_available": "7.84 GB",
            "memory_used": "2.01 GB",
            "total_swap": "2.10 GB",
            "swap_used": "0.00 GB"
        }
        ```
        
4. ### Register a vehicle
    * Request type: POST
    * Route: `/vehicle/register`
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

5. ### Edit a vehicle
    * Request type: POST
    * Route: `/vehicle/edit`
    * Format: JSON

        ```json
        {
            "vid": "644e1ecf1b23abbba13a6f90",
            "company": "Volkswagen",
            "model": "Jetta"
        }
        ```
    * Returns: a JSON object containing the updated vehicle document
        ```json
        {
            "success": "The vehicle was updated",
            "document": {
                "id": {
                "$oid": "644e1ecf1b23abbba13a6f90"
                },
                "company": "Volkswagen",
                "model": "Jetta"
            }
        }
        ```

5. ### Refresh the list of paired vehicles

    * Request type: POST
    * Route: `/vehicle/refresh`
    * Format: JSON

        ```json
        {
            "uid": "<oid of the user>"
        }
        ```
    * Returns: A JSON object containing the count of paired vehicles, along with a list of vehicle objects.

        ```json
        {
            "count": 1,
            "vehicles": [
                {
                "_id": {
                    "$oid": "63613b6e50ddc3b5ef1cca7c"
                },
                "company": "Skoda",
                "model": "Octavia vRS"
                }
            ]
        }
        ```
        
5. ### Create and join a vehicle room

    * Request type: GET
    * Route: `/join/vehicle/{uid}`
    * Format: Plain URL route

        ```
        https://url.com/join/vehicle/{uid}
        ```
        where `uid` is the `$oid` of the vehicle generated during registration
    * Returns: A websocket connection upgrade to the room. The vehicle is in control of the room.
    * Notes: Only one instance of a vehicle should connect at a time. More than one instance of the same vehicle should never attempt to connect and make a room. If this happens, the server may lose contact with the existing instance and will also never connect to the new instance.

6. ### Pair a user & vehicle
    * Request type: GET
    * Route: `/pair/{vid}/{uid}`
    * Format: Plain URL Route
        ```
        https://url.com/pair/{vid}/{uid}?initial="value" 
        ```
        where `uid` & `vid` are the `$oid`'s of the user & vehicle respectively. `value` can be either `true` or `false` (just pass it as a string). It can be obtained from the QR code itself and will indicate whether the vehicle is being paired for the first time or not.
    * Returns: a websocket connection upgrade which automatically disconnects. The disconnect message will contain the result of the database transaction. The vehicle will recieve a message from the server notifying the pair.
    * Notes: As mentioned in #6, we have no way of knowing whether a vehicle is currently hosting a room or not. So, regardless of the vehicle's status, the server will pair the user & vehicle. In order to avoid a false pair situation, make sure to place this request only when you get the vid from the QR code generated by the vehicle itself. Don't attempt connecting manually.

7. ### Join a vehicle's room
    * Request type: GET
    * Route: `/join/user/{vid}/{uid}`
    * Format: Plain URL Route
        ```
        https://url.com/join/user/{vid}/{uid}
        ```
        where `uid` & `vid` are the `$oid`'s of the user & vehicle respectively
    * Returns: a websocket connection upgrade to the room. The vehicle is in control of the room. The vehicle will obey all commands sent by the users. However, a user can't control other users in the room.
    * Notes: If the user isn't paired to the vehicle, the attempt will result in a 404 HTTP response. The connection will only be upgraded if there are no internal errors or conflicts.

8. ### Retrieving logs (daily basis)
    * Request type: POST
    * Route: `/logs/daily`
    * Format: JSON
    
        ```json
        {
            "vid": "<vid of the vehicle>",
            "date": "<date in DD-MM-YYYY format>"
        }
        ```
    
    * Returns: a JSON object with the vehicle stats & report for that day

        ```json
        {
            "average_speed": 65,
            "stress_count": 0,
            "degradation": 0.0,
            "distance_travelled": 10,
            "last_odometer": 56000,
            "max_speed": {
                "speed": 80,
                "hit_at": "07:57 PM"
            }
        }
        ```

9. ### Retrieving logs (periodic basis)
    * Request type: POST
    * Route: `/logs/periodic`
    * Format: JSON
    
        ```json
        {
            "vid": "<vid of the vehicle>",
            "start": "<start date in DD-MM-YYYY format>",
            "end": "<end date in DD-MM-YYYY format>"
        }
        ```
    
    * Returns: a JSON object with the aggregated vehicle stats & report for that period

        ```json
        {
            "average_speed": 65,
            "stress_count": 0,
            "degradation": 0.0,
            "distance_travelled": 100,
            "last_odometer": 56000,
            "max_speed": {
                "speed": 80,
                "hit_at": "07:57 PM"
            }
        }
        ```

10. ### Retrieving logs (overall stats)
    * Request type: POST
    * Route: `/logs/overall`
    * Format: JSON
    
        ```json
        {
            "vid": "<vid of the vehicle>"
        }
        ```
    
    * Returns: a JSON object with the aggregated vehicle stats & report since alpaDrive was initially connected

        ```json
        {
            "average_speed": 65,
            "stress_count": 0,
            "degradation": 0.0,
            "distance_travelled": 100,
            "last_odometer": 56000,
            "max_speed": {
                "speed": 80,
                "hit_at": "07:57 PM"
            }
        }
        ```

## Messaging
The core purpose of this API is to enable organized messaging through websocket connections for connected clients. As such, all messages follow a standard format across the board.

There are two categories of messages that can be sent through the Lobby. Within these categories, messages can be sent to and from the vehicle and connected users. The current format is listed here and is the one being used in v0.1.

### Messaging between clients
Clients can send messages in the modes prescribed here. Any message that doesn't ascertain to the standards of the server will be immediately rejected. As it should be obvious, connected clients can be of two types: **Vehicles** & **Users**. Hence, there are only two possibilities, 

- **Vehicle -> User**: The vehicle should be able to send messages in two ways: Broadcast & Whisper
    - Broadcast: This is when the vehicle has to send a common message to all users
    - Whisper: This is when the vehicle has to send a message to a specific user only without the others knowing
- **User -> Vehicle**: A user will need to send two types of messages: Action & Request
    - Action: This is when the user has to order the vehicle to perform some specific action, say lock the doors.
    - Request: This is when the user has to request certain specific data from the vehicle

As mentioned, here is the basic structure of a valid message. Just keep in mind that even is you don't have to use a certain field for your purpose, don't exclude it as it will violate the standard and the message will be rejected altogether, as mentioned.

```json
{
    "mode": "", // mandatory field. cannot be null
    "vid": "", 
    "conn_id": "",
    "status": "",
    "message": "",
    "attachments": []
}
```

Read on to explore how to use these different modes effectively.

#### Broadcast
This is when the vehicle has to send a common message to all users. This can happen for the following events:

* Internal state of the vehicle has changed & requires all users to update (eg: telemetry)
* A special event occurred in the vehicle & users have to be notified

The general format of such a message is as shown

```json
{
    "mode": "broadcast",
    "vid": "<vehicle_id>", // not mandatory
    "conn_id": "", // blank field
    "status": "success", // one of ["success", "warn", "error"] - defaults to success
    "message": "<message as string>",
    "attachments": [] // optional, as string
}
```

> **Note** The vehicle can broadcast any valid string as a `message`. But, in order for it to be logged, it should follow another standard format.
> ```json
> {
>   "gear": "<integer value>", // optional
>   "speed": "<integer value>", // optional
>   "rpm": "<integer value>", // optional
>   "location": "<string value>", // optional
>   "temp": "<integer value>", // optional
>   "fuel": "<integer value>", // optional
>   "odo": "<integer value>",
>   "stressed": true // or false    
> }
> ```
> If a message is sent without this adhering to this format, then the data won't be logged in the database and will be lost. It won't be use for computation afterwards. The message will however, be broadcasted to the users in the room.

#### Whisper
This is when the vehicle has to send a message to a specific user only without the others knowing. This can happen for the following events:

* A user requested ordered a certain action and only that user needs to know the status
* A user requested some specific data, and it needs to be sent only to that user

The general format of such a message is as shown

```json
{
    "mode": "whisper",
    "vid": "<vehicle_id>", // not mandatory
    "conn_id": "<user_id>", // for verification at the client side
    "status": "success", // one of ["success", "warn", "error"] - defaults to success
    "message": "<message as string>",
    "attachments": [] // optional, as string
}
```

#### Action
This is when the user has to order the vehicle to perform some specific action, say lock the doors. In this case, the action will be a member of a finite predefined set of actions. This should be obvious because a vehicle can only execute so many actions.

The general format of such a message is as shown

```json
{
    "mode": "action",
    "vid": "<vehicle_id>", // optional
    "conn_id": "<user_id>", // for identification be server and/or vehicle
    "status": "success", // one of ["success", "warn", "error"] - defaults to success
    "message": "<action as string>", // one of the predefined actions
    "attachments": [] // optional, as string
}
```

#### Request
This is when the user has to request certain specific data from the vehicle. This can happen when the user taps on some option and the data have to be fetched then. Again, a user can only request so many things to a vehicle, so the request will be a member of a predefined finite set of requests.

The general format of such a message is as shown

```json
{
    "mode": "request",
    "vid": "<vehicle_id>", // optional
    "conn_id": "<user_id>", // for identification be server and/or vehicle
    "status": "success", // one of ["success", "warn", "error"] - defaults to success
    "message": "<request as string>", // one of predefined requests
    "attachments": [] // optional, as string
}
```
### Messages from server
So far, we discussed about how clients can send messages amongst themselves. This part of the guide walks you through the kind of messages that can originate from the server itself. In v0.1, this only occurs due to internal server errors, like when a message isn't compatible with the server's standards. This may change in the future versions. 

A sample message contains the following parameters:

* **event**: denotes the event which caused the message to be sent
* **client**: will hold the data of a client involved with the message, if any
* **message**: an optional message, if any
* **error**: an error message, if any

and will look somewhat like this

```json
{
     "event": "connect" // connect, disconnect, error
     "client": {
         "uid": "<>", // $oid of the client, if involved,
         "conn_id": "<connection uuid>" // uuid representing the agent internally in the server
      },
      "message": "an optional message, if any",
      "error": "an error message, if any"
}
```
## Known Issue(s)

1. ### Accidental pair when vehicle is inactive
    - Description: Due to a core architectural flaw in the design, you can now accidentally pair a user with a vehicle without authorization. There is no way for the pairing mechanism to know whether a vehicle is currently active.
    - Fix: Allow the vehicle to display the pairing QR code **only after confirming it has connected** to the server. That way, no user can send a pair request when the vehicle is inactive.
    - Severity: High
    - Tracked by: Issue #1

Make sure to open a new issue only after confirming it exists in the server and is not a bug in your front-end code.