# Dynamic Programming Templates - Python
# Common DP patterns for competitive programming


def knapsack_01(weights, values, capacity):
    """
    0/1 Knapsack Problem
    Time: O(n * capacity), Space: O(n * capacity)

    Args:
        weights: List of item weights
        values: List of item values
        capacity: Knapsack capacity

    Returns:
        Maximum value achievable
    """
    n = len(weights)
    dp = [[0] * (capacity + 1) for _ in range(n + 1)]

    for i in range(1, n + 1):
        for w in range(capacity + 1):
            # Don't take item i-1
            dp[i][w] = dp[i-1][w]

            # Take item i-1 if it fits
            if weights[i-1] <= w:
                dp[i][w] = max(dp[i][w],
                              dp[i-1][w - weights[i-1]] + values[i-1])

    return dp[n][capacity]


def longest_common_subsequence(s1, s2):
    """
    Longest Common Subsequence (LCS)
    Time: O(m * n), Space: O(m * n)
    """
    m, n = len(s1), len(s2)
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if s1[i-1] == s2[j-1]:
                dp[i][j] = dp[i-1][j-1] + 1
            else:
                dp[i][j] = max(dp[i-1][j], dp[i][j-1])

    return dp[m][n]


def longest_increasing_subsequence(arr):
    """
    Longest Increasing Subsequence (LIS)
    Time: O(n log n), Space: O(n)
    """
    from bisect import bisect_left

    if not arr:
        return 0

    tails = []

    for num in arr:
        pos = bisect_left(tails, num)
        if pos == len(tails):
            tails.append(num)
        else:
            tails[pos] = num

    return len(tails)


def coin_change(coins, amount):
    """
    Minimum coins needed to make amount
    Time: O(amount * n), Space: O(amount)
    """
    dp = [float('inf')] * (amount + 1)
    dp[0] = 0

    for coin in coins:
        for x in range(coin, amount + 1):
            dp[x] = min(dp[x], dp[x - coin] + 1)

    return dp[amount] if dp[amount] != float('inf') else -1


def edit_distance(word1, word2):
    """
    Minimum edit distance (Levenshtein distance)
    Time: O(m * n), Space: O(m * n)
    """
    m, n = len(word1), len(word2)
    dp = [[0] * (n + 1) for _ in range(m + 1)]

    # Initialize base cases
    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j

    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if word1[i-1] == word2[j-1]:
                dp[i][j] = dp[i-1][j-1]
            else:
                dp[i][j] = 1 + min(
                    dp[i-1][j],      # delete
                    dp[i][j-1],      # insert
                    dp[i-1][j-1]     # replace
                )

    return dp[m][n]


def matrix_chain_multiplication(dimensions):
    """
    Matrix chain multiplication - minimum operations
    Time: O(n^3), Space: O(n^2)

    Args:
        dimensions: Array where matrix i has dims[i-1] x dims[i]

    Returns:
        Minimum number of scalar multiplications
    """
    n = len(dimensions) - 1
    dp = [[0] * n for _ in range(n)]

    for length in range(2, n + 1):
        for i in range(n - length + 1):
            j = i + length - 1
            dp[i][j] = float('inf')

            for k in range(i, j):
                cost = (dp[i][k] + dp[k+1][j] +
                       dimensions[i] * dimensions[k+1] * dimensions[j+1])
                dp[i][j] = min(dp[i][j], cost)

    return dp[0][n-1] if n > 0 else 0


# Example usage
if __name__ == "__main__":
    # Knapsack
    weights = [2, 3, 4, 5]
    values = [3, 4, 5, 6]
    capacity = 8
    print(f"Max knapsack value: {knapsack_01(weights, values, capacity)}")

    # LCS
    s1, s2 = "AGGTAB", "GXTXAYB"
    print(f"LCS length: {longest_common_subsequence(s1, s2)}")

    # LIS
    arr = [10, 9, 2, 5, 3, 7, 101, 18]
    print(f"LIS length: {longest_increasing_subsequence(arr)}")

    # Coin change
    coins = [1, 2, 5]
    amount = 11
    print(f"Min coins: {coin_change(coins, amount)}")
