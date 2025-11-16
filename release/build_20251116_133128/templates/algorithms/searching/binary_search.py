# Binary Search Template
# Time Complexity: O(log n)
# Space Complexity: O(1)

def binary_search(arr, target):
    """
    Standard binary search implementation.

    Args:
        arr: Sorted array of integers
        target: Target value to find

    Returns:
        Index of target if found, -1 otherwise
    """
    left, right = 0, len(arr) - 1

    while left <= right:
        mid = left + (right - left) // 2  # Avoid overflow

        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1

    return -1


def binary_search_leftmost(arr, target):
    """
    Find leftmost occurrence of target.
    """
    left, right = 0, len(arr)

    while left < right:
        mid = left + (right - left) // 2
        if arr[mid] < target:
            left = mid + 1
        else:
            right = mid

    return left if left < len(arr) and arr[left] == target else -1


def binary_search_rightmost(arr, target):
    """
    Find rightmost occurrence of target.
    """
    left, right = 0, len(arr)

    while left < right:
        mid = left + (right - left) // 2
        if arr[mid] <= target:
            left = mid + 1
        else:
            right = mid

    return left - 1 if left > 0 and arr[left - 1] == target else -1


# Example usage
if __name__ == "__main__":
    arr = [1, 2, 3, 4, 5, 6, 7, 8, 9]
    print(binary_search(arr, 5))  # Output: 4

    arr_dup = [1, 2, 2, 2, 3, 4, 5]
    print(binary_search_leftmost(arr_dup, 2))   # Output: 1
    print(binary_search_rightmost(arr_dup, 2))  # Output: 3
