;; The topics that heinzelmann should be listening on
(define topics (list "/appliance/2103224640798990846748e1e9662393/publish" "lights/light1" "lights/light2" "custom/meross/set" "custom/meross/plug1/set" "custom/meross/plug2/set"))

;; The timers on which heinzelmann should trigger automations
(define timers (list 
	(TimedEvent "07:00" "timers/alarm")
))

;; Utility function for interacting with this basic TP-Link Kasa Smart Plug to MQTT bridge: https://github.com/Vikum94/WiFi-Plug
(define (set-hs100 number state)
  	(define state_str (if state "on" "off"))
	(send-simple "hs100" (string-append "wifiplug_" (string-append (number->string number) (string-append "_" state_str))))
)

;; Defines action when receiving MQTT message on lights/light1 with payload state_str
(define (lights/light1 state_str) 
	(if (= state_str "off") 
		(set-hs100 1 #false) 
		(set-hs100 1 #true)
	)
)

;; Defines action when receiving MQTT message on lights/light2 with a payload state_str
(define (lights/light2 state_str) 
	(if (= state_str "off") 
		(set-hs100 2 #false) 
		(set-hs100 2 #true)
	)
)

;; Small helper function
(define (bool->on-off b) (if b "on" "off"))

;; These functions aim to make interacting with Meross Smart Plugs more ergonomic
;; In this case, it's specific to the MSS620 model
;; If you want to use this, replace the uuid bits. No guarantee it will work with yours though!
;; For docs on this stuff, see https://github.com/arandall/meross/blob/main/doc/protocol.md
(define (/appliance/2103224640798990846748e1e9662393/publish payload)
	;; Automatically ACK the Meross plug, otherwise it will boot loop (or something similar)
	;; This isn't required for all models, but I only have one that needs this
	(if (string-contains payload "\"namespace\":\"Appliance.Control.Bind\"")
	  	(
			(send-simple "/appliance/2103224640798990846748e1e9662393/subscribe" 
				(replace-str payload "SET" "SETACK")
			)
			(displayln "Meross Appliance.Control.Bind received!")
		)
		;; The plugs will report back their state, this bridges that to a more readable format
		(when
			(and
				(string-contains payload "\"namespace\":\"Appliance.Control.ToggleX\"")
				(string-contains payload "\"method\":\"PUSH\"")
			)
			(
				(send-simple "custom/meross/status" (bool->on-off 
					(string-contains payload "\"channel\":0,\"onoff\":1"))
				)
				(send-simple "custom/meross/plug1/status" (bool->on-off 
					(string-contains payload "\"channel\":1,\"onoff\":1"))
				)
				(send-simple "custom/meross/plug2/status" (bool->on-off 
					(string-contains payload "\"channel\":2,\"onoff\":1"))
				)
			)
		)
	)
)

;; Sends a generic Meross-formatted message
(define (send-meross-message uuid method ability payload)
	(define topic (replace-str "/appliance/UUID/subscribe" "UUID" uuid))
	(define message-id (random-string 32))
	(define timestamp (current-timestamp))
	(define sign (md5 (list message-id timestamp)))

	(define template "{\"header\": {\"messageId\": \"MESSAGEID\", \"namespace\": \"ABILITY\", \"method\": \"METHOD\", \"payloadVersion\": 1, \"from\": \"meross/UUID/0/ack\", \"uuid\": \"UUID\", \"timestamp\": TIMESTAMP, \"timestampMs\": 980, \"sign\": \"SIGN\"}, \"payload\": PAYLOAD}")
	(define mqtt-payload (replace-strs template 
		(list "UUID" "METHOD" "ABILITY" "PAYLOAD" "TIMESTAMP" "MESSAGEID" "SIGN") 
		(list uuid method ability payload timestamp message-id sign)
	))
	(send-simple topic mqtt-payload)
)

;; Sends a ToggleX-type Meross-formatted message
(define (set-meross uuid channel state)
  	(define channel_str (number->string channel))
	(define state_str (if state "1" "0"))
	(define template "{\"togglex\": {\"channel\": CHANNEL, \"onoff\": ONOFF}}")
	(define payload (replace-strs template 
		(list "CHANNEL" "ONOFF")
		(list channel_str state_str)
	))
	(send-meross-message uuid "SET" "Appliance.Control.ToggleX" payload)
)

;; These three functions are triggered via MQTT events and allow for easier control of the Meross Plug
(define (custom/meross/set payload)
  	(define state (= payload "on" ))
	(set-meross "2103224640798990846748e1e9662393" 0 state)
)

(define (custom/meross/plug1/set payload)
  	(define state (= payload "on" ))
	(set-meross "2103224640798990846748e1e9662393" 1 state)
)

(define (custom/meross/plug2/set payload)
  	(define state (= payload "on" ))
	(set-meross "2103224640798990846748e1e9662393" 2 state)
)

;; Defines action when timer timers/alarm is triggered
(define (timers/alarm _)
	(set-hs100 1 #true)
)

