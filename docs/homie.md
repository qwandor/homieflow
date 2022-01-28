# Homie integration

Homieflow acts as a [Homie](https://homieiot.github.io/) controller, exposing supported Homie
devices to Google Home.

## Configuration

First configure Google login in your `server.toml`:

```toml
[logins.google]
client-id = "my-client.apps.googleusercontent.com"
```

Then for each user, configure the MQTT broker and Homie topic prefix:

```toml
[[users]]
id = "uuidabc123"
username = "someexampleuser"
email = "someexampleuser@gmail.com"
admin = false
homie = { host = "mqtt.myserver.example", port = 8883, use-tls = true, username = "exampleuser", password = "somemqttpassword", client-id = "homieflow_exampleuser", homie-prefix = "homie", reconnect-interval-seconds = 600 }
```

## Device mapping

The Homieflow server will map Homie device nodes to Google Home devices, depending on their
properties. Currently it supports these types:

| Google Home device type | Google Home device trait | Homie property id | Homie data type  | Notes                                                                                               |
| ----------------------- | ------------------------ | ----------------- | ---------------- | --------------------------------------------------------------------------------------------------- |
| Switch                  | OnOff                    | `on`              | boolean          |                                                                                                     |
| Light                   | OnOff                    | `on`              | boolean          | Must also have a `brightness` or `color` property to be recognised as a light rather than a switch. |
|                         | Brightness               | `brightness`      | integer or float | Optional. Must include a `$format` specifying the range.                                            |
|                         | ColorSetting             | `color`           | color            | Optional. Both RGB and HSV are supported.                                                           |
| Thermostat              | TemperatureSetting       | `temperature`     | integer or float | Temperature is assumed to be in °C.                                                                 |
|                         |                          | `humidity`        | integer or float | Optional.                                                                                           |
