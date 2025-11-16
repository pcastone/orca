// Smooth scrolling for anchor links
document.addEventListener('DOMContentLoaded', function() {
    // Smooth scroll for all anchor links
    const links = document.querySelectorAll('a[href^="#"]');
    links.forEach(link => {
        link.addEventListener('click', function(e) {
            e.preventDefault();
            const targetId = this.getAttribute('href').substring(1);
            const targetElement = document.getElementById(targetId);

            if (targetElement) {
                const offset = 80; // Account for sticky header
                const elementPosition = targetElement.getBoundingClientRect().top;
                const offsetPosition = elementPosition + window.pageYOffset - offset;

                window.scrollTo({
                    top: offsetPosition,
                    behavior: 'smooth'
                });

                // Update active state in sidebar
                updateActiveSidebarLink(this);
            }
        });
    });

    // Update active sidebar link based on scroll position
    const sections = document.querySelectorAll('section[id]');
    const sidebarLinks = document.querySelectorAll('.sidebar-link');

    function updateActiveLinkOnScroll() {
        let currentSection = '';

        sections.forEach(section => {
            const sectionTop = section.offsetTop - 100;
            const sectionHeight = section.clientHeight;

            if (window.pageYOffset >= sectionTop && window.pageYOffset < sectionTop + sectionHeight) {
                currentSection = section.getAttribute('id');
            }
        });

        sidebarLinks.forEach(link => {
            link.classList.remove('active');
            if (link.getAttribute('href') === `#${currentSection}`) {
                link.classList.add('active');
            }
        });
    }

    // Function to update sidebar link active state
    function updateActiveSidebarLink(clickedLink) {
        sidebarLinks.forEach(link => {
            link.classList.remove('active');
        });

        if (clickedLink.classList.contains('sidebar-link')) {
            clickedLink.classList.add('active');
        }
    }

    // Update on scroll
    window.addEventListener('scroll', updateActiveLinkOnScroll);

    // Initial update
    updateActiveLinkOnScroll();

    // Add copy button to code blocks
    const codeBlocks = document.querySelectorAll('pre code');
    codeBlocks.forEach(block => {
        const pre = block.parentElement;

        // Create copy button
        const copyButton = document.createElement('button');
        copyButton.className = 'absolute top-2 right-2 px-3 py-1 text-xs bg-gray-700 hover:bg-gray-600 text-white rounded transition-colors duration-200';
        copyButton.textContent = 'Copy';

        // Make pre element relative for button positioning
        pre.style.position = 'relative';

        // Add click handler
        copyButton.addEventListener('click', async () => {
            const code = block.textContent;

            try {
                await navigator.clipboard.writeText(code);
                copyButton.textContent = 'Copied!';
                copyButton.classList.add('bg-green-600');
                copyButton.classList.remove('bg-gray-700');

                setTimeout(() => {
                    copyButton.textContent = 'Copy';
                    copyButton.classList.remove('bg-green-600');
                    copyButton.classList.add('bg-gray-700');
                }, 2000);
            } catch (err) {
                console.error('Failed to copy code:', err);
                copyButton.textContent = 'Failed';

                setTimeout(() => {
                    copyButton.textContent = 'Copy';
                }, 2000);
            }
        });

        pre.appendChild(copyButton);
    });

    // Mobile menu toggle (if needed in future)
    const setupMobileMenu = () => {
        const mobileMenuButton = document.getElementById('mobile-menu-button');
        const mobileMenu = document.getElementById('mobile-menu');

        if (mobileMenuButton && mobileMenu) {
            mobileMenuButton.addEventListener('click', () => {
                mobileMenu.classList.toggle('hidden');
            });
        }
    };

    setupMobileMenu();

    // Add keyboard navigation for accessibility
    document.addEventListener('keydown', (e) => {
        // Alt + Arrow keys for navigation
        if (e.altKey) {
            if (e.key === 'ArrowLeft') {
                // Go to previous page
                const currentPage = window.location.pathname.split('/').pop();
                if (currentPage === 'llm.html') {
                    window.location.href = 'index.html';
                } else if (currentPage === 'tooling.html') {
                    window.location.href = 'llm.html';
                }
            } else if (e.key === 'ArrowRight') {
                // Go to next page
                const currentPage = window.location.pathname.split('/').pop();
                if (currentPage === 'index.html') {
                    window.location.href = 'llm.html';
                } else if (currentPage === 'llm.html') {
                    window.location.href = 'tooling.html';
                }
            }
        }
    });

    // Add search functionality (basic implementation)
    const addSearchHighlight = () => {
        const urlParams = new URLSearchParams(window.location.search);
        const searchTerm = urlParams.get('search');

        if (searchTerm) {
            const content = document.querySelector('main');
            if (content) {
                highlightText(content, searchTerm);
            }
        }
    };

    function highlightText(element, searchTerm) {
        const regex = new RegExp(`(${searchTerm})`, 'gi');
        const walker = document.createTreeWalker(
            element,
            NodeFilter.SHOW_TEXT,
            null,
            false
        );

        const nodesToReplace = [];
        while (walker.nextNode()) {
            const node = walker.currentNode;
            if (regex.test(node.textContent)) {
                nodesToReplace.push(node);
            }
        }

        nodesToReplace.forEach(node => {
            const span = document.createElement('span');
            span.innerHTML = node.textContent.replace(
                regex,
                '<mark class="bg-yellow-200">$1</mark>'
            );
            node.parentNode.replaceChild(span, node);
        });
    }

    addSearchHighlight();
});

// Console log for debugging
console.log('acolib documentation loaded');
