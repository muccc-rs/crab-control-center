from gql import Client, gql
from gql.transport.websockets import WebsocketsTransport

SUBSCRIPTION = gql(
    """\
subscription WatchHardware {
  watch(period: 0.2) {
    inputs {
      pressureFullscale
    }
  }
}
"""
)


def main():
    transport = WebsocketsTransport(
        url="ws://crabcontrol.rahix.net:8080/graphql-subscriptions",
        subprotocols=[WebsocketsTransport.APOLLO_SUBPROTOCOL],
        connect_args={"ping_interval": None},
    )

    client = Client(transport=transport)

    for result in client.subscribe(SUBSCRIPTION):
        pressure = (result["watch"]["inputs"]["pressureFullscale"]) / (65535)
        progress = "#" * (int(pressure * 5 * 70))
        print(f"{pressure:.4f} {progress}")


if __name__ == "__main__":
    main()
