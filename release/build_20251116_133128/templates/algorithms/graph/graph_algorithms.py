# Graph Algorithms Template - Python
# Common graph algorithms for competitive programming

from collections import deque, defaultdict
import heapq


class Graph:
    """Graph representation with common algorithms."""

    def __init__(self, vertices):
        self.V = vertices
        self.graph = defaultdict(list)

    def add_edge(self, u, v, weight=1):
        """Add edge from u to v with optional weight."""
        self.graph[u].append((v, weight))

    def bfs(self, start):
        """
        Breadth-First Search
        Time: O(V + E), Space: O(V)
        """
        visited = [False] * self.V
        queue = deque([start])
        visited[start] = True
        result = []

        while queue:
            vertex = queue.popleft()
            result.append(vertex)

            for neighbor, _ in self.graph[vertex]:
                if not visited[neighbor]:
                    visited[neighbor] = True
                    queue.append(neighbor)

        return result

    def dfs(self, start):
        """
        Depth-First Search (iterative)
        Time: O(V + E), Space: O(V)
        """
        visited = [False] * self.V
        stack = [start]
        result = []

        while stack:
            vertex = stack.pop()

            if not visited[vertex]:
                visited[vertex] = True
                result.append(vertex)

                for neighbor, _ in self.graph[vertex]:
                    if not visited[neighbor]:
                        stack.append(neighbor)

        return result

    def dijkstra(self, start):
        """
        Dijkstra's shortest path algorithm
        Time: O((V + E) log V), Space: O(V)
        """
        dist = [float('inf')] * self.V
        dist[start] = 0
        pq = [(0, start)]  # (distance, vertex)

        while pq:
            d, u = heapq.heappop(pq)

            if d > dist[u]:
                continue

            for v, weight in self.graph[u]:
                if dist[u] + weight < dist[v]:
                    dist[v] = dist[u] + weight
                    heapq.heappush(pq, (dist[v], v))

        return dist

    def topological_sort(self):
        """
        Topological sort using DFS (for DAG)
        Time: O(V + E), Space: O(V)
        """
        visited = [False] * self.V
        stack = []

        def dfs_util(v):
            visited[v] = True
            for neighbor, _ in self.graph[v]:
                if not visited[neighbor]:
                    dfs_util(neighbor)
            stack.append(v)

        for i in range(self.V):
            if not visited[i]:
                dfs_util(i)

        return stack[::-1]


def bellman_ford(vertices, edges, start):
    """
    Bellman-Ford algorithm (handles negative weights)
    Time: O(VE), Space: O(V)

    Args:
        vertices: Number of vertices
        edges: List of (u, v, weight) tuples
        start: Starting vertex

    Returns:
        distances array or None if negative cycle exists
    """
    dist = [float('inf')] * vertices
    dist[start] = 0

    # Relax edges V-1 times
    for _ in range(vertices - 1):
        for u, v, weight in edges:
            if dist[u] != float('inf') and dist[u] + weight < dist[v]:
                dist[v] = dist[u] + weight

    # Check for negative cycles
    for u, v, weight in edges:
        if dist[u] != float('inf') and dist[u] + weight < dist[v]:
            return None  # Negative cycle detected

    return dist


def floyd_warshall(adj_matrix):
    """
    Floyd-Warshall all-pairs shortest path
    Time: O(V^3), Space: O(V^2)

    Args:
        adj_matrix: 2D adjacency matrix (use float('inf') for no edge)

    Returns:
        2D matrix of shortest distances
    """
    V = len(adj_matrix)
    dist = [row[:] for row in adj_matrix]  # Deep copy

    for k in range(V):
        for i in range(V):
            for j in range(V):
                dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])

    return dist


# Example usage
if __name__ == "__main__":
    g = Graph(6)
    g.add_edge(0, 1, 4)
    g.add_edge(0, 2, 2)
    g.add_edge(1, 2, 1)
    g.add_edge(1, 3, 5)
    g.add_edge(2, 3, 8)
    g.add_edge(2, 4, 10)
    g.add_edge(3, 4, 2)
    g.add_edge(3, 5, 6)
    g.add_edge(4, 5, 3)

    print("BFS from 0:", g.bfs(0))
    print("DFS from 0:", g.dfs(0))
    print("Dijkstra from 0:", g.dijkstra(0))
