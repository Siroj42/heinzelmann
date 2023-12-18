;; The topics that heinzelmann should be listening on
(define topics (list "lights/light1" "lights/light2"))

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

;; Defines action when timer timers/alarm is triggered
(define (timers/alarm _)
	(set-hs100 1 #true)
)

