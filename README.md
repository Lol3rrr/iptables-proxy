# IPtables-Proxy
A Reverse Proxy utilizing IPtables to route the inoming packets to the correct end points.

## Intended Use-Case
This is intended to make it easier to expose services running behind a VPN, by easily allowing you to
forward external packets to a given address inside of the network.

## Example Setup
In this example we consider 3 Nodes involved, a Proxy, a Server and a Client.

The Proxy is running this software listening on some internal address.

The Server is connected to the Proxy over a VPN and is not reachable from the Client directly.
When the Server wants to expose some service running on it to the Client, it sends a corresponding
`Create` Request to `iptables-proxy` running on the Proxy, which will then setup the corresponding
iptables rules to forward external traffic to that service.

When the Client now sends some packets to the Proxy on the given IP+Port, the Packets get fowarded
to the Server using their VPN connection and all returning packets are also again forwarded to the Client
through the Proxy.

When the Server stops the Service it has exposed, the given Route can be removed by sending a corresponding
`Remove` Request to `iptables-proxy` running on the Proxy, which will delete the associated IPtables rules
and thereby stop forwarding packets.
