# NoTrust-Track
Linux Agent  which tracks and logs all incoming and outgoing TCP and UDP connections along with the name of the process and who owns the process. NoTrust-Track is a userspace tool and doesn't require the installation of any propriety kernel modules. It leverages the iptables ip_conntrack module. 

Currently it supports output to TCP and UDP Syslog, ElasticSearch and output to the NoTrust Server.



## Installation

### Deb
To install simply install the released debian package, it should in


## Configuration
By default the configuration is kept in __/etc/notrust/config.yaml__

The settings are

* __name__ Defines the human readable name you want to give the agent, if you don't provide one, a girls name will be given to the agent.
* __uuid__ Defines the UUID for this agent, if you don't provide one a random UUID will be generated.
* __directory__ Defines the data directory which the agent will use, typically this is set to /usr/share/notrust
* __outputs__ Defines where the output should be sent.
  * __syslog__ For syslog output
    * __Localhost__ To output straight to the local syslog
    * __TCP__ For TCP Syslog output
    * __UDP__ For UDP Syslog output
  * __elasticsearch__ For ES output, you have to provide the ES URL Plus the index, for instance: "http://my.elasticserch.node.notrust.com:9200/my_index"
  * __notrust_endpoint__ To pipe to the NoTrust Server, provide the URL for your NoTrust Server.
* __filters__ Defines the connections which NoTrust-Track should not report on.
  * __non_process_connections__ - By setting this to false, you will catch all connections, including multicast. This can be noisy and not particularly useful.
  * __dns_requests__ - By setting this to false, you will get all DNS look ups on 53 and 5353, this can be very noisy.
  
  * __notrust_track_connections__ - By setting this to false we will report on connections which the NoTrust-Track daemon makes, if you have an output defined which is network based (i.e. ES, TCP, UDP Syslog) this can create a infinite loop of reporting =)
  
  
## Example of Output
__Open Connection__ - When a connection is opened the following output is given,  the hash is derived by the properties of the connection and can be matched to the corresponding close.

```javascript
{
  "uuid":"b2f0281d-da73-4116-8639-8a1c693511b0",
  "agent":"b15da2a9-67dd-446c-82ce-9512174bc16f",
  "hash" : 950265093776986234,
  "timestamp" : "2018-10-22T10:40:34.763563458+00:00",
  "protocol" : "TCP",
  "source" : "172.16.144.102",
  "destination" : "104.197.3.80",
  "source_port" : 59325,
  "destination_port" : 80,
  "username" : "root",
  "uid" : 0, 
  "program_details" : {
    "inode" : 631905,
    "pid" : 656,
    "process_name" : "NetworkManager",
    "command_line" : [
      "/usr/sbin/NetworkManager",
      "--no-daemon"
    ]
  }
}
```

__Close Connection__ 
```javascript
{
  "uuid":"b2f0281d-da73-4116-8639-8a1c693511b0",
  "agent":"b15da2a9-67dd-446c-82ce-9512174bc16f",
  "hash" : 1334410269481100237,
  "timestamp" : "2018-10-22T10:07:36.651838320+00:00",
  "protocol" : "TCP",
  "source" : "172.16.144.102",
  "destination" : "104.198.143.177",
  "source_port" : 50351,
  "destination_port" : 80
}
```

## Notes
In order for NoTrust-Track to work, it requires the ip_conntrack module to be loaded. If you find that no connections are being reported, please try running 

```bash
sudo modprobe ip_conntrack
```

And this should resolve it.  

As NoTrust-Track does NOT run as root by default, it requires the following capabilities to run:
cap_sys_ptrace, cap_net_admin, cap_dac_read_search

These can be set by using

```bash  
setcap 'cap_sys_ptrace,cap_net_admin,cap_dac_read_search=+ep' /usr/sbin/notrust-track
```
