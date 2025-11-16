// Simple client-side search functionality
document.addEventListener('DOMContentLoaded', function() {
  const searchInput = document.getElementById('search-input');
  const searchResults = document.getElementById('search-results');
  const searchOverlay = document.getElementById('search-overlay');

  if (!searchInput) return;

  // Search index - will be populated from all pages
  const searchIndex = [];

  // Debounce function
  function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
      const later = function() {
        clearTimeout(timeout);
        func(...args);
      };
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
    };
  }

  // Search function
  function performSearch(query) {
    if (!query || query.length < 2) {
      hideResults();
      return;
    }

    const lowerQuery = query.toLowerCase();
    const results = searchIndex.filter(function(item) {
      return item.title.toLowerCase().includes(lowerQuery) ||
             item.content.toLowerCase().includes(lowerQuery) ||
             item.keywords.some(function(keyword) {
               return keyword.toLowerCase().includes(lowerQuery);
             });
    }).slice(0, 10); // Limit to 10 results

    displayResults(results, query);
  }

  // Display search results
  function displayResults(results, query) {
    if (!searchResults) return;

    if (results.length === 0) {
      searchResults.innerHTML = '<div class="p-4 text-gray-600">No results found for "' + escapeHtml(query) + '"</div>';
    } else {
      let html = '<div class="divide-y divide-gray-200">';
      results.forEach(function(result) {
        const snippet = getSnippet(result.content, query);
        html += '<a href="' + result.url + '" class="block p-4 hover:bg-gray-50 transition-colors">';
        html += '<div class="font-medium text-gray-900">' + escapeHtml(result.title) + '</div>';
        html += '<div class="text-sm text-gray-600 mt-1">' + snippet + '</div>';
        html += '<div class="text-xs text-primary-600 mt-1">' + result.category + '</div>';
        html += '</a>';
      });
      html += '</div>';
      searchResults.innerHTML = html;
    }

    showResults();
  }

  // Get snippet of content with highlighted query
  function getSnippet(content, query, maxLength = 150) {
    const lowerContent = content.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const index = lowerContent.indexOf(lowerQuery);

    if (index === -1) {
      return escapeHtml(content.substring(0, maxLength)) + '...';
    }

    const start = Math.max(0, index - 50);
    const end = Math.min(content.length, index + maxLength);
    let snippet = content.substring(start, end);

    if (start > 0) snippet = '...' + snippet;
    if (end < content.length) snippet = snippet + '...';

    // Highlight the query
    const regex = new RegExp('(' + escapeRegex(query) + ')', 'gi');
    snippet = escapeHtml(snippet).replace(regex, '<mark class="bg-yellow-200">$1</mark>');

    return snippet;
  }

  // Escape HTML
  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  // Escape regex special characters
  function escapeRegex(text) {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  // Show results
  function showResults() {
    if (searchResults) searchResults.classList.remove('hidden');
    if (searchOverlay) searchOverlay.classList.remove('hidden');
  }

  // Hide results
  function hideResults() {
    if (searchResults) searchResults.classList.add('hidden');
    if (searchOverlay) searchOverlay.classList.add('hidden');
  }

  // Event listeners
  if (searchInput) {
    searchInput.addEventListener('input', debounce(function(e) {
      performSearch(e.target.value);
    }, 300));

    searchInput.addEventListener('focus', function() {
      if (this.value.length >= 2) {
        performSearch(this.value);
      }
    });
  }

  if (searchOverlay) {
    searchOverlay.addEventListener('click', function() {
      hideResults();
    });
  }

  // Close search on Escape key
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      hideResults();
      if (searchInput) searchInput.blur();
    }
  });

  // Initialize search index with current page
  function indexCurrentPage() {
    const title = document.querySelector('h1')?.textContent || document.title;
    const content = document.querySelector('main')?.textContent || '';
    const category = document.querySelector('[data-category]')?.dataset.category || 'Documentation';
    const keywords = document.querySelector('meta[name="keywords"]')?.getAttribute('content')?.split(',') || [];

    searchIndex.push({
      title: title.trim(),
      content: content.replace(/\s+/g, ' ').trim().substring(0, 500),
      url: window.location.pathname,
      category: category,
      keywords: keywords.map(function(k) { return k.trim(); })
    });
  }

  indexCurrentPage();

  // Load search index from external file if available
  fetch('/search-index.json')
    .then(function(response) {
      if (response.ok) {
        return response.json();
      }
      throw new Error('Search index not found');
    })
    .then(function(data) {
      searchIndex.length = 0; // Clear current index
      searchIndex.push(...data);
    })
    .catch(function(error) {
      console.log('Using local search index only:', error.message);
    });
});
