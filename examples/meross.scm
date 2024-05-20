;; The topics that heinzelmann should be listening on
(map subscribe (list "/appliance/2103224640798990846748e1e9662393/publish"
                     "custom/meross/set"
                     "custom/meross/plug1/set"
                     "custom/meross/plug2/set"))

;; Small helper functions
(define (bool->on-off b) (if b "on" "off"))
(define (number->on-off i) (bool->on-off (not (= i 0))))

;; These functions aim to make interacting with Meross Smart Plugs more ergonomic
;; In this case, it's specific to the MSS620 model
;; If you want to use this, replace the uuid bits. No guarantee it will work with yours though!
;; For docs on this stuff, see https://github.com/arandall/meross/blob/main/doc/protocol.md

(register-event! "/appliance/2103224640798990846748e1e9662393/publish" 
   (lambda x 
     (let* ((payload (string->jsexpr (cadr x)))
            (header (hash-get payload 'header)))
       (cond ((string=? (hash-get header 'namespace) "Appliance.Control.Bind")
              ; Automatically ACK a new Meross plug, otherwise it will boot loop (or something similar)
              ; This isn't required for all models, but I only have one that needs this
              (send-simple "/appliance/2103224640798990846748e1e9662393/subscribe"
                           (value->jsexpr-string (hash-insert payload 'header (hash-insert header 'method "SETACK"))))
              (displayln "Meross Appliance.Control.Bind received and ACKed on /appliance/2103224640798990846748e1e9662393/publish!"))
             ((and (string=? (hash-get header 'namespace) "Appliance.Control.ToggleX")
                   (string=? (hash-get header 'method) "PUSH"))
              (send-meross-status "custom/meross" (hash-get (hash-get payload 'payload)
                                                            'togglex)))))))

;; Sends a more readable Meross status message, iterating over all of the plug's channels
(define (send-meross-status base-topic channels)
  (when (not (empty? channels))
    (let* ((channel (car channels))
           (topic (string-append base-topic (match (hash-get channel 'channel)
                                             (0.0 "/status")
                                             (1.0 "/plug1/status")
                                             (2.0 "/plug2/status")))))
      (send-retain topic (number->on-off (hash-get channel 'onoff))
                    (send-meross-status base-topic (cdr channels))))))

;; Sends a generic Meross-formatted message
(define (send-meross-message uuid method ability payload)
  (let* ((message-id (random-string 32))
         (timestamp (int->string (current-timestamp)))
         (topic (string-append "/appliance/" uuid "/subscribe"))
         (payload-hash (hash 'header (hash 'messageId message-id
                                           'namespace ability
                                           'method method
                                           'payloadVersion 1
                                           'from (string-append "meross/" uuid "/0/ack")
                                           'uuid uuid
                                           'timestamp timestamp
                                           'timestampMs 980
                                           'sign (md5 (list message-id timestamp)))
                             'payload payload)))
    (send-simple topic (value->jsexpr-string payload-hash))))

;; Sends a ToggleX-type Meross-formatted message
(define (set-meross uuid channel state)
  (let* ((state_int (if state 1 0))
         (payload (hash 'togglex (hash
                                  'channel channel
                                  'onoff state_int))))
    (send-meross-message uuid "SET" "Appliance.Control.ToggleX" payload)))

;; These three functions are triggered via MQTT events and allow for easier control of the Meross Plug
(register-event! "custom/meross/set"
  (lambda x
    (let* ((state (string=? (cadr x) "on")))
      (set-meross "2103224640798990846748e1e9662393" 0 state))))

(register-event! "custom/meross/plug1/set"
  (lambda x
    (let ((state (string=? (cadr x) "on")))
      (set-meross "2103224640798990846748e1e9662393" 1 state))))

(register-event! "custom/meross/plug2/set"
  (lambda x
    (let ((state (string=? (cadr x) "on")))
      (set-meross "2103224640798990846748e1e9662393" 2 state))))
