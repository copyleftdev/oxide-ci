from diagrams import Cluster, Diagram, Edge
from diagrams.onprem.ci import TravisCI # Fallback/Generic CI icon or similar
from diagrams.onprem.database import PostgreSQL
from diagrams.onprem.queue import Nats
from diagrams.onprem.compute import Server
from diagrams.onprem.container import Docker
from diagrams.programming.language import Rust
from diagrams.custom import Custom
from diagrams.aws.general import User

# Theme Colors (Oxide Style)
graph_attr = {
    "fontsize": "20",
    "bgcolor": "transparent"
}

with Diagram("Oxide CI Architecture", show=False, filename="docs/media/architecture", graph_attr=graph_attr):
    user = User("Developer")
    
    with Cluster("Control Plane"):
        api = Server("Oxide API")
        scheduler = Server("Oxide Scheduler")
        db = PostgreSQL("Oxide DB")
        bus = Nats("Event Bus (NATS)")

        user >> Edge(label="CLI / UI") >> api
        api >> Edge(label="State") >> db
        api >> Edge(label="Events") >> bus
        
        scheduler << Edge(label="Consume") << bus
        scheduler >> Edge(label="Read/Write") >> db
        
    with Cluster("Execution Plane (Agents)"):
        with Cluster("Oxide Agent Node"):
            agent = Server("Agent")
            runner = Docker("Runner (Docker/Nix)")
            plugins = Rust("Native Plugins")
            
            agent << Edge(label="Job Assign") << bus
            agent >> Edge(label="Logs/Status") >> bus
            agent >> runner
            agent >> plugins
