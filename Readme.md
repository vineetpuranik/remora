  # Remora                                                                 
   
  A remora is a fish that attaches itself to a larger host, inspecting and 
  filtering everything that passes through — a fitting name for a Zero
  Trust proxy.                                                             
                  
  ## What we're building

  A Rust prototype of a Zero Trust network proxy with three components:    
  - **Control Plane** — gRPC server that enforces access policy and
  issues/validates JWT tokens                                              
  - **TCP Proxy** — L4 forwarder that checks policy before passing traffic
  to a backend                                                             
  - **Client** — CLI tool to issue tokens and test access decisions
                                                                           
  ## What we're learning                                                   
   
  - Async Rust with `tokio` — task spawning, bidirectional I/O, async      
  traits          
  - gRPC with `tonic` + `prost` — protobuf codegen, service implementation,
   interceptors                                                            
  - Concurrent state with `DashMap` — lock-free reads over `Mutex<HashMap>`
  - JWT auth with `jsonwebtoken` — token issuance, validation, and error   
  propagation                                                              
  - Rust ownership fundamentals — `Arc`, `Send + Sync`, `Result` error     
  handling                                                                 
  - Observability — structured tracing with `tracing-subscriber`, task
  profiling with `tokio-console`  