;; The topics that heinzelmann should be listening on
(map subscribe (list "lights/light1" "lights/light2"))

;; Utility function for interacting with this basic TP-Link Kasa Smart Plug to MQTT bridge: https://github.com/Vikum94/WiFi-Plug
(define (set-hs100 number state)
  (define state_str (if state "on" "off"))
  (send-simple "hs100" 
               (string-append "wifiplug_" 
                              (number->string number) 
                              "_" 
                              state_str)))

(register-timer! "alarm"
  (lambda () (set-hs100 1 #true)))
(set-timer "07:00" "alarm")

;; Defines action when receiving MQTT message on lights/light1 with payload state_str
(register-event! "lights/light1" 
  (lambda (_ state_str)
    (if (string=? state_str "off") 
      (set-hs100 1 #false) 
      (set-hs100 1 #true))))
    
;; Defines action when receiving MQTT message on lights/light2 with a payload state_str
(register-event! "lights/light2" 
  (lambda (_ state_str)
    (if (string=? state_str "off") 
      (set-hs100 2 #false) 
      (set-hs100 2 #true))))
